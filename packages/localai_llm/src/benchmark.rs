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
	/// Measurement duration after warm-up, for example 500ms, 10s, or 1m.
	#[arg(long, value_name = "TIME", default_value = "10s")]
	duration: humantime::Duration,
    /// The largest number of generated tokens of one generation.
    #[arg(long, default_value_t = 128, value_parser = clap::value_parser!(u64).range(1..))]
    max_tokens: u64,
    /// Print JSON instead of the table.
    #[arg(long)]
    json: bool,
}

/// The benchmark always takes the most probable token, so that each run generates the same text.
const BENCHMARK_SAMPLER: SamplerSettings = SamplerSettings {
    temperature: 0.0,
    top_k: 0,
    repeat_penalty: 1.0,
    seed: 0,
};

#[allow(non_snake_case)]
pub(crate) fn run(arguments: Arguments, config: LlmConfig, output: Option<PathBuf>) -> Result<()> {
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
    let engine = LlmEngine::load(config.clone())?;
    let model_load_seconds = started.elapsed().as_secs_f64();
	let request = GenerationRequest {
		max_tokens: arguments.max_tokens as usize,
		sampler: BENCHMARK_SAMPLER,
		..GenerationRequest::new("Write three sentences about the sea.")
	};
	let warmup = Instant::now();
	engine.generate(&request, |_| {})?;
	let warmupSeconds = warmup.elapsed().as_secs_f64();
	eprintln!("Measuring generations for {}...", arguments.duration);
	let started = Instant::now();
	let measurement = Measurement::_measure(
		duration,
		|| engine.generate(&request, |_| {}),
		|| started.elapsed(),
	)?;
    write_report(
        &Report {
            package: "localai_llm",
            model: config.model.clone(),
            backend,
            max_tokens: arguments.max_tokens,
            model_load_seconds,
			durationSeconds: duration.as_secs_f64(),
			warmupSeconds,
			measurement,
            peak_process_resident_bytes: peak_memory(),
            memory_scope: "Process resident-memory high-water mark over model loading, warmup and inference; separate GPU allocations are not measured",
        },
        &arguments,
        output.as_deref(),
    )
}

/// Timing of the completed generations in one measurement.
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
#[allow(non_snake_case)]
struct Report {
    package: &'static str,
    model: String,
    backend: &'static str,
    max_tokens: u64,
    model_load_seconds: f64,
	/// Requested measurement duration in seconds.
	#[serde(rename = "duration_seconds")]
	durationSeconds: f64,
	/// Seconds spent on the generation that precedes the timed measurements.
	#[serde(rename = "warmup_seconds")]
	warmupSeconds: f64,
	/// Timing and token counts for the completed generations.
	#[serde(flatten)]
	measurement: Measurement,
    peak_process_resident_bytes: Option<u64>,
    memory_scope: &'static str,
}

impl Measurement {
	/// Runs generations until the elapsed callback reaches the requested duration.
	/// Returns the completed count, elapsed time and phase rates, or the first generation error.
	#[allow(non_snake_case)]
	fn _measure(
		duration: Duration,
		mut generateFn: impl FnMut() -> Result<Generation>,
		mut elapsedFn: impl FnMut() -> Duration,
	) -> Result<Self> {
		anyhow::ensure!(duration.is_zero() == false, "The benchmark duration must be greater than zero");
		let mut completedGenerations = 0;
		let mut promptTokens = 0;
		let mut generatedTokens = 0;
		let mut promptSeconds = 0.0;
		let mut generationSeconds = 0.0;
		let totalSeconds = loop {
			let generation = generateFn()?;
			completedGenerations += 1;
			promptTokens += generation.prompt_tokens;
			generatedTokens += generation.generated_tokens;
			promptSeconds += generation.prompt_seconds;
			generationSeconds += generation.generation_seconds;
			let elapsed = elapsedFn();
			if elapsed >= duration {
				break elapsed.as_secs_f64();
			}
		};
		anyhow::ensure!(
			promptSeconds > 0.0 && generationSeconds > 0.0,
			"The measured prompt time and generation time must be greater than zero"
		);
		Ok(Self {
			completed_generations: completedGenerations,
			prompt_tokens: promptTokens / completedGenerations as usize,
			generated_tokens: generatedTokens / completedGenerations as usize,
			total_seconds: totalSeconds,
			seconds_per_generation: totalSeconds / completedGenerations as f64,
			prompt_tokens_per_second: promptTokens as f64 / promptSeconds,
			generated_tokens_per_second: generatedTokens as f64 / generationSeconds,
		})
	}
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
		"Measured {} generations of at most {} tokens in {:.3} seconds after warm-up.",
		report.measurement.completed_generations, report.max_tokens, report.measurement.total_seconds
	);
	println!(
		"{:>13} {:>16} {:>18} {:>16} {:>19}",
		"Prompt tokens", "Generated tokens", "Seconds/generation", "Prompt tokens/s", "Generated tokens/s"
	);
	println!(
		"{:>13} {:>16} {:>18.3} {:>16.1} {:>19.1}",
		report.measurement.prompt_tokens,
		report.measurement.generated_tokens,
		report.measurement.seconds_per_generation,
		report.measurement.prompt_tokens_per_second,
		report.measurement.generated_tokens_per_second
	);
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
        let result = Measurement::_measure(
            Duration::from_secs(8),
            || {
                calls.set(calls.get() + 1);
                Ok(generation(100, 50, 0.5, 2.5))
            },
            || Duration::from_secs(calls.get() * 3),
        )
        .unwrap();
        assert_eq!(calls.get(), 3);
        assert_eq!(result.completed_generations, 3);
        assert_eq!(result.total_seconds, 9.0);
        assert_eq!(result.prompt_tokens, 100);
        assert_eq!(result.generated_tokens, 50);
        assert_eq!(result.seconds_per_generation, 3.0);
        assert_eq!(result.prompt_tokens_per_second, 200.0);
        assert_eq!(result.generated_tokens_per_second, 20.0);
    }

    #[test]
    fn benchmark_reports_errors_instead_of_counting_failed_generations() {
        let calls = Cell::new(0);
        let result = Measurement::_measure(
            Duration::from_secs(8),
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 2 {
                    anyhow::bail!("device failed");
                }
                Ok(generation(10, 10, 1.0, 1.0))
            },
            || Duration::from_secs(calls.get() * 2),
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 2);
    }

	/// Checks that zero duration is rejected before starting work.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkRejectsZeroDuration() {
		assert!(Measurement::_measure(Duration::ZERO, || panic!("must not run"), || Duration::ZERO).is_err());
	}
}
