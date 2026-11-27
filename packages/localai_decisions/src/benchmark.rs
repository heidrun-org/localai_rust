//! Speed measurements owned by the Qwen3.5 command line tool.
use crate::outputFormat::OutputFormat;
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use localai_decisions::{DecisionConfig, DecisionEngine, Device};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Args)]
#[allow(non_snake_case)]
pub(crate) struct Arguments {
	/// Total measurement time across selected decision types, excluding warm-up (for example 500ms, 10s, or 1m).
	#[arg(long, value_name = "TIME", default_value = "10s")]
	duration: humantime::Duration,
	/// Decision type to measure. Example: benchmark --type score --duration 1s.
	#[arg(long = "type", value_enum, default_value = "all")]
	decisionType: DecisionType,
    /// Print JSON instead of the table.
    #[arg(long)]
    json: bool,
}

/// Decision types accepted by the benchmark.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
enum DecisionType {
	/// Choice among named options.
	Choice,
	/// Yes-or-no decision.
	Noul,
	/// Choice among ordered levels.
	Score,
	/// Measure each supported type.
	All,
}

/// A decision type and its representative request.
struct Case {
	/// Decision type used for filtering and reporting.
	kind: DecisionType,
    request: Value,
}

#[allow(non_snake_case)]
pub(crate) fn run(
    arguments: Arguments,
    config: DecisionConfig,
    output: Option<PathBuf>,
	format: Option<OutputFormat>,
) -> Result<()> {
	let duration = Duration::from(arguments.duration);
	anyhow::ensure!(duration.is_zero() == false, "The benchmark duration must be greater than zero");
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
	let selectedCases: Vec<_> = Case::_builtIn()
		.into_iter()
		.filter(|case| arguments.decisionType == DecisionType::All || case.kind == arguments.decisionType)
		.collect();
	let durationPerCase = (duration / selectedCases.len() as u32).max(Duration::from_nanos(1));
	let mut results = Vec::with_capacity(selectedCases.len());
	for case in selectedCases {
        let warmup = Instant::now();
        let response = engine.decide(&case.request)?;
        let warmup_seconds = warmup.elapsed().as_secs_f64();
        let input_tokens = response["usage"]["input_tokens"]
            .as_u64()
            .context("The model returned no input token count")?
            as usize;
		eprintln!(
			"Measuring {:?} for {}...",
			case.kind,
			humantime::format_duration(durationPerCase),
		);
        let started = Instant::now();
        let measurement = Measurement::_measure(
            durationPerCase,
            || {
                engine.decide(&case.request)?;
                Ok(())
            },
            || started.elapsed(),
        )?;
        results.push(CaseResult {
            case: case.kind,
            input_tokens,
            warmup_seconds,
            measurement,
        });
    }
    write_report(&Report {
        package: "localai_decisions", model: config.model.clone(), backend,
        durationSeconds: duration.as_secs_f64(), model_load_seconds, cases: results,
        peak_process_resident_bytes: peak_memory(),
        memory_scope: "Process resident-memory high-water mark over model loading, warmup and inference; separate GPU allocations are not measured",
    }, &arguments, output.as_deref(), format)
}

impl Case {
	/// Returns the requests for the three decision types on the same text.
	#[allow(non_snake_case)]
	fn _builtIn() -> [Self; 3] {
		let text = "I was charged twice for my order. Please refund the extra payment today.";
		[
			(
				DecisionType::Choice,
				json!({
					"type": "choice",
					"instructions": "Which team should handle this?",
					"criteria": {"billing": null, "shipping": null, "technical": null},
				}),
			),
			(
				DecisionType::Noul,
				json!({"type": "noul", "instructions": "Does the customer ask for a refund?"}),
			),
			(
				DecisionType::Score,
				json!({
					"type": "score",
					"instructions": "How urgent is this request?",
					"criteria": ["can wait", "this week", "today", "immediately"],
				}),
			),
		]
		.map(|(kind, question)| Self {
			kind,
			request: json!({"state": text, "questions": {"decision": question}}),
		})
	}
}

/// Timing of the completed decisions in one measurement.
#[derive(Debug, Serialize)]
struct Measurement {
    completed_decisions: u64,
    total_seconds: f64,
    seconds_per_decision: f64,
    decisions_per_second: f64,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case: DecisionType,
    input_tokens: usize,
    warmup_seconds: f64,
    #[serde(flatten)]
    measurement: Measurement,
}

#[derive(Debug, Serialize)]
#[allow(non_snake_case)]
struct Report {
    package: &'static str,
    model: String,
    backend: &'static str,
	/// Requested total measurement time across selected decision types, in seconds.
	#[serde(rename = "duration_seconds")]
	durationSeconds: f64,
    model_load_seconds: f64,
    cases: Vec<CaseResult>,
    peak_process_resident_bytes: Option<u64>,
    memory_scope: &'static str,
}

impl Measurement {
	/// Runs decisions until the elapsed callback reaches the requested duration.
	/// Returns the completed count and rates, or the first decision error.
	#[allow(non_snake_case)]
	fn _measure(
		duration: Duration,
		mut decideFn: impl FnMut() -> Result<()>,
		mut elapsedFn: impl FnMut() -> Duration,
	) -> Result<Self> {
		anyhow::ensure!(duration.is_zero() == false, "The benchmark duration must be greater than zero");
		let mut completedDecisions = 0;
		let totalSeconds = loop {
			decideFn()?;
			completedDecisions += 1;
			let elapsed = elapsedFn();
			if elapsed >= duration {
				break elapsed.as_secs_f64();
			}
		};
		Ok(Self {
			completed_decisions: completedDecisions,
			total_seconds: totalSeconds,
			seconds_per_decision: totalSeconds / completedDecisions as f64,
			decisions_per_second: completedDecisions as f64 / totalSeconds,
		})
	}
}

#[allow(non_snake_case)]
fn write_report(
    report: &Report,
    arguments: &Arguments,
    output: Option<&std::path::Path>,
	format: Option<OutputFormat>,
) -> Result<()> {
	if let Some(outputFormat) = format {
		return outputFormat.write(report, output);
	}
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
        "Requested {:.3} seconds of total measurement time across selected decision types.",
        report.durationSeconds
    );
    println!(
        "{:<15} {:>12} {:>10} {:>13} {:>16} {:>16}",
        "Decision type", "Input tokens", "Decisions", "Total seconds", "Seconds/decision", "Decisions/second"
    );
    for result in &report.cases {
        println!(
            "{:<15?} {:>12} {:>10} {:>13.3} {:>16.4} {:>16.2}",
            result.case,
            result.input_tokens,
            result.measurement.completed_decisions,
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
        let result = Measurement::_measure(
            Duration::from_secs(10),
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
        let result = Measurement::_measure(
            Duration::from_secs(10),
            || {
                clock.set(clock.get() + 6000);
                Ok(())
            },
            || Duration::from_millis(clock.get()),
        )
        .unwrap();
        assert_eq!(result.completed_decisions, 2);
        assert_eq!(result.total_seconds, 12.0);
        assert_eq!(result.seconds_per_decision, 6.0);
        assert_eq!(result.decisions_per_second, 1.0 / 6.0);
    }

    #[test]
    fn benchmark_reports_errors_instead_of_counting_failed_decisions() {
        let calls = Cell::new(0);
        let result = Measurement::_measure(
            Duration::from_secs(10),
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 3 {
                    anyhow::bail!("device failed");
                }
                Ok(())
            },
            || Duration::from_secs(calls.get()),
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 3);
    }

	/// Checks that zero duration is rejected before starting work.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkRejectsZeroDuration() {
		assert!(Measurement::_measure(Duration::ZERO, || panic!("must not run"), || Duration::ZERO).is_err());
	}

	/// Checks that each decision type asks one question about the existing benchmark input.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkUsesOneTextForAllDecisionTypes() {
		let cases = Case::_builtIn();
		assert_eq!(cases.each_ref().map(|case| case.kind),
			[DecisionType::Choice, DecisionType::Noul, DecisionType::Score]);
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
