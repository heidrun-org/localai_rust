//! Speed measurements owned by the Qwen3.5 command line tool.
use anyhow::{Context, Result};
use clap::Args;
use localai_decisions::{DecisionConfig, DecisionEngine, Device};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Args)]
pub(crate) struct Arguments {
    /// Completed decisions per decision type, after loading and warmup.
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..))]
    decisions: u64,
    /// Print JSON instead of the table.
    #[arg(long)]
    json: bool,
}

struct Case {
    name: &'static str,
    request: Value,
}

pub(crate) fn run(
    arguments: Arguments,
    config: DecisionConfig,
    output: Option<PathBuf>,
) -> Result<()> {
    let backend = match config.device {
        Device::Cpu => "cpu",
        Device::Gpu => {
            if cfg!(feature = "metal") {
                "metal"
            } else if cfg!(feature = "cuda") {
                "cuda"
            } else if cfg!(feature = "vulkan") {
                "vulkan"
            } else {
                "gpu"
            }
        }
    };
    eprintln!("Loading {} on {backend} before timing...", config.model);
    let started = Instant::now();
    let engine = DecisionEngine::load(config.clone())?;
    let model_load_seconds = started.elapsed().as_secs_f64();
    let mut results = Vec::new();
    for case in Case::_builtIn() {
        let warmup = Instant::now();
        let response = engine.decide(&case.request)?;
        let warmup_seconds = warmup.elapsed().as_secs_f64();
        let input_tokens = response["usage"]["input_tokens"]
            .as_u64()
            .context("The model returned no input token count")?
            as usize;
        eprintln!(
            "Measuring {}: {} decisions...",
            case.name, arguments.decisions
        );
        let started = Instant::now();
        let measurement = measure_decisions(
            arguments.decisions,
            || {
                engine.decide(&case.request)?;
                Ok(())
            },
            || started.elapsed(),
        )?;
        results.push(CaseResult {
            case: case.name,
            input_tokens,
            warmup_seconds,
            measurement,
        });
    }
    write_report(&Report {
        package: "localai_decisions", model: config.model.clone(), backend,
        decisions_per_case: arguments.decisions, model_load_seconds, cases: results,
        peak_process_resident_bytes: peak_memory(),
        memory_scope: "Process resident-memory high-water mark over model loading, warmup and inference; separate GPU allocations are not measured",
    }, &arguments, output.as_deref())
}

impl Case {
	/// Returns the requests for the three decision types on the same text.
	#[allow(non_snake_case)]
	fn _builtIn() -> [Self; 3] {
		let text = "I was charged twice for my order. Please refund the extra payment today.";
		[
			(
				"Choice",
				json!({
					"type": "choice",
					"instructions": "Which team should handle this?",
					"criteria": {"billing": null, "shipping": null, "technical": null},
				}),
			),
			(
				"Noul",
				json!({"type": "noul", "instructions": "Does the customer ask for a refund?"}),
			),
			(
				"Score",
				json!({
					"type": "score",
					"instructions": "How urgent is this request?",
					"criteria": ["can wait", "this week", "today", "immediately"],
				}),
			),
		]
		.map(|(name, question)| Self {
			name,
			request: json!({"state": text, "questions": {"decision": question}}),
		})
	}
}

/// Timing of a fixed number of fully completed decisions.
#[derive(Debug, Serialize)]
struct Measurement {
    completed_decisions: u64,
    total_seconds: f64,
    seconds_per_decision: f64,
    decisions_per_second: f64,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case: &'static str,
    input_tokens: usize,
    warmup_seconds: f64,
    #[serde(flatten)]
    measurement: Measurement,
}

#[derive(Debug, Serialize)]
struct Report {
    package: &'static str,
    model: String,
    backend: &'static str,
    decisions_per_case: u64,
    model_load_seconds: f64,
    cases: Vec<CaseResult>,
    peak_process_resident_bytes: Option<u64>,
    memory_scope: &'static str,
}

fn measure_decisions(
    decisions: u64,
    mut decide: impl FnMut() -> Result<()>,
    mut elapsed: impl FnMut() -> Duration,
) -> Result<Measurement> {
    anyhow::ensure!(
        decisions > 0,
        "The decision count must be greater than zero"
    );
    for _ in 0..decisions {
        decide()?;
    }
    let total_seconds = elapsed().as_secs_f64();
    anyhow::ensure!(
        total_seconds > 0.0,
        "The measured elapsed time must be greater than zero"
    );
    Ok(Measurement {
        completed_decisions: decisions,
        total_seconds,
        seconds_per_decision: total_seconds / decisions as f64,
        decisions_per_second: decisions as f64 / total_seconds,
    })
}

fn write_report(
    report: &Report,
    arguments: &Arguments,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    if let Some(path) = output {
        std::fs::write(path, format!("{json}\n"))
            .with_context(|| format!("Could not write {}", path.display()))?;
    }
    if arguments.json {
        println!("{json}");
        return Ok(());
    }
    println!("Model: {}", report.model);
    println!("Backend: {}", report.backend);
    println!(
        "Model loaded in {:.2} seconds (excluded from timing)",
        report.model_load_seconds
    );
    println!(
        "Measured {} completed decisions per decision type after warmup.",
        report.decisions_per_case
    );
    println!(
        "{:<24} {:>12} {:>13} {:>16} {:>16}",
        "Decision type", "Input tokens", "Total seconds", "Seconds/decision", "Decisions/second"
    );
    for result in &report.cases {
        println!(
            "{:<24} {:>12} {:>13.3} {:>16.4} {:>16.2}",
            result.case,
            result.input_tokens,
            result.measurement.total_seconds,
            result.measurement.seconds_per_decision,
            result.measurement.decisions_per_second
        );
    }
    if let Some(bytes) = report.peak_process_resident_bytes {
        println!(
            "Peak process resident memory: {:.1} MiB",
            bytes as f64 / 1_048_576.0
        );
    }
    if let Some(path) = output {
        println!("JSON: {}", path.display());
    }
    Ok(())
}

fn peak_memory() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        // Safety: getrusage initializes this correctly sized and aligned structure on success.
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        let value = u64::try_from(usage.ru_maxrss).ok()?;
        #[cfg(target_os = "macos")]
        return Some(value);
        #[cfg(not(target_os = "macos"))]
        return Some(value * 1024);
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn benchmark_completes_five_decisions_and_calculates_average_time_and_rate() {
        let clock = Cell::new(0);
        let calls = Cell::new(0);
        let result = measure_decisions(
            5,
            || {
                calls.set(calls.get() + 1);
                clock.set(clock.get() + 2000);
                Ok(())
            },
            || Duration::from_millis(clock.get()),
        )
        .unwrap();
        assert_eq!(calls.get(), 5);
        assert_eq!(result.completed_decisions, 5);
        assert_eq!(result.total_seconds, 10.0);
        assert_eq!(result.seconds_per_decision, 2.0);
        assert_eq!(result.decisions_per_second, 0.5);
    }

    #[test]
    fn benchmark_waits_for_slow_decisions_instead_of_returning_zero() {
        let clock = Cell::new(0);
        let result = measure_decisions(
            5,
            || {
                clock.set(clock.get() + 6000);
                Ok(())
            },
            || Duration::from_millis(clock.get()),
        )
        .unwrap();
        assert_eq!(result.completed_decisions, 5);
        assert_eq!(result.total_seconds, 30.0);
        assert_eq!(result.seconds_per_decision, 6.0);
        assert_eq!(result.decisions_per_second, 1.0 / 6.0);
    }

    #[test]
    fn benchmark_reports_errors_instead_of_counting_failed_decisions() {
        let calls = Cell::new(0);
        let result = measure_decisions(
            5,
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 3 {
                    anyhow::bail!("device failed");
                }
                Ok(())
            },
            || Duration::from_secs(10),
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn benchmark_rejects_zero_decisions_and_zero_elapsed_time() {
        assert!(measure_decisions(0, || panic!("must not run"), || Duration::ZERO).is_err());
        assert!(measure_decisions(5, || Ok(()), || Duration::ZERO).is_err());
    }

	/// Checks that each decision type asks one question about the existing benchmark input.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkUsesOneTextForAllDecisionTypes() {
		let cases = Case::_builtIn();
		assert_eq!(cases.each_ref().map(|case| case.name), ["Choice", "Noul", "Score"]);
		for (case, decisionType) in cases.iter().zip(["choice", "noul", "score"]) {
			assert_eq!(
				case.request["state"],
				"I was charged twice for my order. Please refund the extra payment today.",
			);
			let questions = case.request["questions"].as_object().unwrap();
			assert_eq!(questions.len(), 1);
			assert_eq!(questions["decision"]["type"], decisionType);
		}
	}
}
