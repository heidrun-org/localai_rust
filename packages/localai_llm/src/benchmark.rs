//! Speed measurements of the command line tool.
use anyhow::{Context, Result};
use clap::Args;
use localai_llm::{Device, Generation, GenerationRequest, LlmConfig, LlmEngine, SamplerSettings};
use serde::Serialize;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Args)]
pub(crate) struct Arguments {
    /// Completed generations per case, after loading and warmup.
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u64).range(1..))]
    generations: u64,
    /// The largest number of generated tokens of one generation.
    #[arg(long, default_value_t = 128, value_parser = clap::value_parser!(u64).range(1..))]
    max_tokens: u64,
    /// Print JSON instead of the table.
    #[arg(long)]
    json: bool,
}

struct Case {
    name: &'static str,
    prompt: String,
}

/// The benchmark always takes the most probable token, so that each run generates the same text.
const BENCHMARK_SAMPLER: SamplerSettings = SamplerSettings {
    temperature: 0.0,
    top_k: 0,
    repeat_penalty: 1.0,
    seed: 0,
};

pub(crate) fn run(arguments: Arguments, config: LlmConfig, output: Option<PathBuf>) -> Result<()> {
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
    let engine = LlmEngine::load(config.clone())?;
    let model_load_seconds = started.elapsed().as_secs_f64();
    let mut results = Vec::new();
    for case in cases() {
        let request = GenerationRequest {
            max_tokens: arguments.max_tokens as usize,
            sampler: BENCHMARK_SAMPLER,
            ..GenerationRequest::new(case.prompt.clone())
        };
        let warmup = Instant::now();
        engine.generate(&request, |_| {})?;
        let warmup_seconds = warmup.elapsed().as_secs_f64();
        eprintln!("Measuring {}: {} generations...", case.name, arguments.generations);
        let started = Instant::now();
        let measurement = measure_generations(arguments.generations, || engine.generate(&request, |_| {}), || started.elapsed())?;
        results.push(CaseResult {
            case: case.name,
            warmup_seconds,
            measurement,
        });
    }
    write_report(
        &Report {
            package: "localai_llm",
            model: config.model.clone(),
            backend,
            generations_per_case: arguments.generations,
            max_tokens: arguments.max_tokens,
            model_load_seconds,
            cases: results,
            peak_process_resident_bytes: peak_memory(),
            memory_scope: "Process resident-memory high-water mark over model loading, warmup and inference; separate GPU allocations are not measured",
        },
        &arguments,
        output.as_deref(),
    )
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "short",
            prompt: SHORT_PROMPT.to_string(),
        },
        Case {
            name: "long",
            prompt: format!("{}\n\n{LONG_QUESTION}", LONG_PARAGRAPH.repeat(8)),
        },
    ]
}

/// Timing of a fixed number of fully completed generations.
#[derive(Debug, Serialize)]
struct Measurement {
    completed_generations: u64,
    prompt_tokens: usize,
    generated_tokens: usize,
    total_seconds: f64,
    seconds_per_generation: f64,
    prompt_tokens_per_second: f64,
    generated_tokens_per_second: f64,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case: &'static str,
    warmup_seconds: f64,
    #[serde(flatten)]
    measurement: Measurement,
}

#[derive(Debug, Serialize)]
struct Report {
    package: &'static str,
    model: String,
    backend: &'static str,
    generations_per_case: u64,
    max_tokens: u64,
    model_load_seconds: f64,
    cases: Vec<CaseResult>,
    peak_process_resident_bytes: Option<u64>,
    memory_scope: &'static str,
}

const SHORT_PROMPT: &str = "Write three sentences about the sea.";
const LONG_PARAGRAPH: &str = "I contacted customer support about the duplicate payment for my order. \
    The first payment was accepted and the order arrived last week, but my bank statement shows \
    another charge for the same amount. The online account shows only one purchase. \
    Please check the billing records and refund the extra payment. I need the money today. ";
const LONG_QUESTION: &str = "Write a reply to this customer message.";

/// Runs `count` generations, adds their token counts and times, and divides by the total time.
fn measure_generations(
    count: u64,
    mut generate: impl FnMut() -> Result<Generation>,
    mut elapsed: impl FnMut() -> Duration,
) -> Result<Measurement> {
    anyhow::ensure!(count > 0, "The generation count must be greater than zero");
    let mut prompt_tokens = 0;
    let mut generated_tokens = 0;
    let mut prompt_seconds = 0.0;
    let mut generation_seconds = 0.0;
    for _ in 0..count {
        let generation = generate()?;
        prompt_tokens += generation.prompt_tokens;
        generated_tokens += generation.generated_tokens;
        prompt_seconds += generation.prompt_seconds;
        generation_seconds += generation.generation_seconds;
    }
    let total_seconds = elapsed().as_secs_f64();
    anyhow::ensure!(total_seconds > 0.0, "The measured elapsed time must be greater than zero");
    anyhow::ensure!(
        prompt_seconds > 0.0 && generation_seconds > 0.0,
        "The measured prompt time and generation time must be greater than zero"
    );
    Ok(Measurement {
        completed_generations: count,
        prompt_tokens: prompt_tokens / count as usize,
        generated_tokens: generated_tokens / count as usize,
        total_seconds,
        seconds_per_generation: total_seconds / count as f64,
        prompt_tokens_per_second: prompt_tokens as f64 / prompt_seconds,
        generated_tokens_per_second: generated_tokens as f64 / generation_seconds,
    })
}

fn write_report(report: &Report, arguments: &Arguments, output: Option<&std::path::Path>) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    if let Some(path) = output {
        std::fs::write(path, format!("{json}\n")).with_context(|| format!("Could not write {}", path.display()))?;
    }
    if arguments.json {
        println!("{json}");
        return Ok(());
    }
    println!("Model: {}", report.model);
    println!("Backend: {}", report.backend);
    println!("Model loaded in {:.2} seconds (excluded from timing)", report.model_load_seconds);
    println!(
        "Each case is warmed up, then {} completed generations of at most {} tokens are measured.",
        report.generations_per_case, report.max_tokens
    );
    println!(
        "{:<8} {:>13} {:>16} {:>18} {:>16} {:>19}",
        "Case", "Prompt tokens", "Generated tokens", "Seconds/generation", "Prompt tokens/s", "Generated tokens/s"
    );
    for result in &report.cases {
        println!(
            "{:<8} {:>13} {:>16} {:>18.3} {:>16.1} {:>19.1}",
            result.case,
            result.measurement.prompt_tokens,
            result.measurement.generated_tokens,
            result.measurement.seconds_per_generation,
            result.measurement.prompt_tokens_per_second,
            result.measurement.generated_tokens_per_second
        );
    }
    if let Some(bytes) = report.peak_process_resident_bytes {
        println!("Peak process resident memory: {:.1} MiB", bytes as f64 / 1_048_576.0);
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
    use localai_llm::StopReason;
    use std::cell::Cell;

    fn generation(prompt_tokens: usize, generated_tokens: usize, prompt_seconds: f64, generation_seconds: f64) -> Generation {
        Generation {
            text: String::new(),
            prompt_tokens,
            generated_tokens,
            stop_reason: StopReason::MaxTokens,
            prompt_seconds,
            generation_seconds,
        }
    }

    #[test]
    fn benchmark_completes_three_generations_and_calculates_the_rates() {
        let calls = Cell::new(0);
        let result = measure_generations(
            3,
            || {
                calls.set(calls.get() + 1);
                Ok(generation(100, 50, 0.5, 2.5))
            },
            || Duration::from_secs(9),
        )
        .unwrap();
        assert_eq!(calls.get(), 3);
        assert_eq!(result.completed_generations, 3);
        assert_eq!(result.prompt_tokens, 100);
        assert_eq!(result.generated_tokens, 50);
        assert_eq!(result.seconds_per_generation, 3.0);
        assert_eq!(result.prompt_tokens_per_second, 200.0);
        assert_eq!(result.generated_tokens_per_second, 20.0);
    }

    #[test]
    fn benchmark_reports_errors_instead_of_counting_failed_generations() {
        let calls = Cell::new(0);
        let result = measure_generations(
            3,
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 2 {
                    anyhow::bail!("device failed");
                }
                Ok(generation(10, 10, 1.0, 1.0))
            },
            || Duration::from_secs(10),
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn benchmark_rejects_zero_generations_and_zero_elapsed_time() {
        assert!(measure_generations(0, || panic!("must not run"), || Duration::ZERO).is_err());
        assert!(measure_generations(3, || Ok(generation(10, 10, 1.0, 1.0)), || Duration::ZERO).is_err());
    }

    #[test]
    fn benchmark_cases_have_a_short_and_a_long_prompt() {
        let cases = cases();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].name, "short");
        assert_eq!(cases[1].name, "long");
        assert!(cases[1].prompt.len() > cases[0].prompt.len() * 20);
    }
}
