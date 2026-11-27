//! Command line for local text embeddings.

#[path = "outputFormat.rs"]
mod outputFormat;

use outputFormat::OutputFormat;
use anyhow::{bail, Context, Result};
use clap::Parser;
use localai_embeddings::{
    cosine_similarity, Backend, ChunkingMode, DocumentEmbedding, EmbeddingConfig, EmbeddingEngine,
    ModelLoadTimings, DEFAULT_MODEL_ID,
};
use serde::Serialize;
use std::io::{IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(
    name = "localai_embeddings",
    about = "Compute text embeddings locally"
)]
struct Arguments {
	/// Select text, Markdown, or JSON output.
	#[arg(short = 'f', long, value_enum, global = true, value_name = "FORMAT")]
	format: Option<OutputFormat>,
    /// Hugging Face identifier for a supported BERT sentence embedding model.
    #[arg(long, default_value = DEFAULT_MODEL_ID)]
    model: String,
    /// Check Hugging Face for model updates and download newer files if available.
    #[arg(long)]
    check_model_updates: bool,
    /// Local inference backend.
    #[arg(long, value_enum, default_value = "cpu")]
    backend: Backend,
    /// Text to embed. Repeat for multiple inputs.
    #[arg(long = "text", short = 't')]
    texts: Vec<String>,
    /// UTF-8 text file to embed. Repeat for multiple files.
    #[arg(long = "file")]
    files: Vec<PathBuf>,
    /// Document structure. Auto selects a mode from each file extension.
    #[arg(long, value_enum, default_value = "auto")]
    chunking: ChunkingMode,
    /// Maximum tokens repeated from the previous chunk. Defaults to no overlap.
    #[arg(long, default_value_t = 0)]
    overlap_tokens: usize,
    /// Compare the global vectors of exactly two inputs.
    #[arg(long, conflicts_with = "benchmark")]
    compare: bool,
	/// Measure embedding throughput after one warm-up round.
	#[arg(long)]
	benchmark: bool,
	/// Benchmark duration after warm-up, for example 500ms, 10s, or 1m.
	#[arg(long, value_name = "TIME", default_value = "10s", requires = "benchmark")]
	duration: humantime::Duration,
    /// CPU worker count. Defaults to the inference library's automatic selection.
    #[arg(long)]
    threads: Option<NonZeroUsize>,
    /// Save the result to a file instead of standard output.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Print progress and timing details to standard error.
    #[arg(short, long)]
    verbose: bool,
}

struct Input {
    source: String,
    text: String,
    mode: ChunkingMode,
}

#[derive(Serialize)]
struct DocumentOutput {
    input_index: usize,
    source: String,
    #[serde(flatten)]
    embedding: DocumentEmbedding,
}

#[derive(Serialize)]
struct EmbeddingOutput<'a> {
    model: &'a str,
    backend: &'static str,
    dimensions: usize,
    documents: Vec<DocumentOutput>,
}

#[derive(Serialize)]
struct SimilarityOutput<'a> {
    model: &'a str,
    backend: &'static str,
    dimensions: usize,
    first_text: &'a str,
    second_text: &'a str,
    cosine_similarity: f32,
}

#[derive(Serialize)]
/// Model settings and measured throughput for the supplied text.
#[allow(non_snake_case)]
struct BenchmarkOutput<'a> {
	/// Primary throughput in mebibytes of original text per second.
	input_mib_per_second: f64,
	/// Requested measurement duration in seconds.
	#[serde(rename = "duration_seconds")]
	durationSeconds: f64,
	/// Model loading time in seconds.
	#[serde(rename = "model_load_seconds")]
	modelLoadSeconds: f64,
	/// Time to process the warm-up round in seconds.
	#[serde(rename = "warmup_seconds")]
	warmupSeconds: f64,
	/// Total bytes of original text in completed timed rounds.
	#[serde(rename = "total_input_bytes")]
	totalInputBytes: u128,
    model: &'a str,
    backend: &'static str,
    build_profile: &'static str,
    cpu_threads: usize,
    dimensions: usize,
    max_tokens: usize,
    rounds: usize,
    chunks_per_round: usize,
    input_bytes_per_round: usize,
    chunk_tokens_per_round: usize,
    padded_tokens_per_round: usize,
    elapsed_seconds: f64,
    embeddings_per_second: f64,
    input_bytes_per_second: f64,
    chunk_tokens_per_second: f64,
    padded_tokens_per_second: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

#[allow(non_snake_case)]
fn run() -> Result<()> {
    let arguments = Arguments::parse();
	let duration = Duration::from(arguments.duration);
	if arguments.benchmark == true && duration.is_zero() == true {
		bail!("The benchmark duration must be greater than zero");
	}
    // Set the worker count before tokenization or inference initializes worker pools.
    if let Some(threads) = arguments.threads {
        std::env::set_var("RAYON_NUM_THREADS", threads.to_string());
        std::env::set_var("CANDLE_NUM_THREADS", threads.to_string());
    }
    let inputs = read_inputs(&arguments)?;
    if arguments.compare && inputs.len() != 2 {
        bail!("`--compare` requires exactly two text inputs or files")
    }
    if arguments.verbose == true || arguments.benchmark == true {
        eprintln!(
            "Model files: {}.",
            if arguments.check_model_updates {
                "check Hugging Face for updates"
            } else {
                "use cached files; download only missing files"
            }
        );
        eprintln!(
            "Loading model `{}` on the `{}` backend...",
            arguments.model,
            arguments.backend.as_str()
        );
    }
    let model_load_started = Instant::now();
    let (mut engine, load_timings) = EmbeddingEngine::load_with_timings(EmbeddingConfig {
        model_id: arguments.model,
        backend: arguments.backend,
        check_model_updates: arguments.check_model_updates,
    })?;
	let modelLoadSeconds = model_load_started.elapsed().as_secs_f64();
    if arguments.verbose {
        print_load_timings(&load_timings);
        eprintln!(
            "Loaded `{}` ({} dimensions, maximum {} tokens) in {:.2} seconds.",
            engine.model_id(),
            engine.dimensions(),
            engine.max_tokens(),
            modelLoadSeconds
        );
        eprintln!(
            "Computing chunk and global vectors for {} input(s)...",
            inputs.len()
        );
    }
    let embedding_started = Instant::now();
    let output = if arguments.benchmark == true {
		BenchmarkOutput::measure(&mut engine, &inputs, arguments.overlap_tokens, duration, modelLoadSeconds)?
    } else {
        let documents = compute_documents(&mut engine, &inputs, arguments.overlap_tokens)?;
        if arguments.verbose {
            let chunks = documents
                .iter()
                .map(|document| document.embedding.chunks.len())
                .sum::<usize>();
            eprintln!(
                "Computed {chunks} chunk embedding(s) and {} global vector(s) in {:.2} seconds.",
                documents.len(),
                embedding_started.elapsed().as_secs_f64()
            );
        }
        if arguments.compare {
            serde_json::to_vec_pretty(&SimilarityOutput {
                model: engine.model_id(),
                backend: engine.backend().as_str(),
                dimensions: engine.dimensions(),
                first_text: &inputs[0].text,
                second_text: &inputs[1].text,
                cosine_similarity: cosine_similarity(
                    &documents[0].embedding.global_vector,
                    &documents[1].embedding.global_vector,
                )?,
            })?
        } else {
            serde_json::to_vec_pretty(&EmbeddingOutput {
                model: engine.model_id(),
                backend: engine.backend().as_str(),
                dimensions: engine.dimensions(),
                documents,
            })?
        }
    };
	let output = match arguments.format {
		Some(outputFormat @ (OutputFormat::Text | OutputFormat::Markdown)) => {
			let value: serde_json::Value = serde_json::from_slice(&output)?;
			outputFormat.render(
				&value,
				arguments.output.is_none() && std::io::stdout().is_terminal(),
			)?.into_bytes()
		}
		Some(OutputFormat::Json) | None => output,
	};
    if let Some(path) = arguments.output {
        std::fs::write(&path, output)
            .with_context(|| format!("Could not write `{}`", path.display()))?;
        eprintln!("Saved embedding output to {}", path.display());
    } else {
        std::io::stdout().write_all(&output)?;
        if output.ends_with(b"\n") == false {
            println!();
        }
    }
    Ok(())
}

fn compute_documents(
    engine: &mut EmbeddingEngine,
    inputs: &[Input],
    overlap: usize,
) -> Result<Vec<DocumentOutput>> {
    inputs
        .iter()
        .enumerate()
        .map(|(input_index, input)| {
            Ok(DocumentOutput {
                input_index,
                source: input.source.clone(),
                embedding: engine.embed_document(&input.text, input.mode, overlap)?,
            })
        })
        .collect()
}

impl BenchmarkOutput<'_> {
	/// Warms up and then measures complete passes over the supplied inputs.
	/// Uses the selected chunk overlap and duration, and returns the report as JSON bytes.
	#[allow(non_snake_case)]
	fn measure(
		engine: &mut EmbeddingEngine,
		inputs: &[Input],
		overlap: usize,
		duration: Duration,
		modelLoadSeconds: f64,
	) -> Result<Vec<u8>> {
		eprintln!("Warming up the embeddings benchmark with {} input(s)...", inputs.len());
		let warmupStarted = Instant::now();
		let warmup = compute_documents(engine, inputs, overlap)?;
		let warmupSeconds = warmupStarted.elapsed().as_secs_f64();
		let chunks = warmup
			.iter()
			.map(|document| document.embedding.chunks.len())
			.sum::<usize>();
		let chunkTokens = warmup
			.iter()
			.flat_map(|document| &document.embedding.chunks)
			.map(|chunk| chunk.chunk.token_count)
			.sum::<usize>();
		let paddedTokens = warmup
			.iter()
			.map(|document| {
				document
					.embedding
					.chunks
					.chunks(8)
					.map(|batch| {
						batch
							.iter()
							.map(|chunk| chunk.chunk.token_count)
							.max()
							.unwrap_or(0)
							* batch.len()
					})
					.sum::<usize>()
			})
			.sum::<usize>();
		drop(warmup);

		let bytes = inputs.iter().map(|input| input.text.len()).sum::<usize>();
		eprintln!("Measuring embedding throughput for {}...", humantime::format_duration(duration));
		let started = Instant::now();
		let (rounds, elapsed) = Self::_measureRounds(
			duration,
			|| {
				std::hint::black_box(compute_documents(engine, inputs, overlap)?);
				Ok(())
			},
			|| started.elapsed(),
		)?;
		let elapsedSeconds = elapsed.as_secs_f64();
		let totalInputBytes = bytes as u128 * rounds as u128;
		let bytesPerSecond = totalInputBytes as f64 / elapsedSeconds;
		let roundsPerSecond = rounds as f64 / elapsedSeconds;
		serde_json::to_vec_pretty(&BenchmarkOutput {
			input_mib_per_second: bytesPerSecond / (1024.0 * 1024.0),
			durationSeconds: duration.as_secs_f64(),
			modelLoadSeconds,
			warmupSeconds,
			totalInputBytes,
			model: engine.model_id(),
			backend: engine.backend().as_str(),
			dimensions: engine.dimensions(),
			max_tokens: engine.max_tokens(),
			build_profile: if cfg!(debug_assertions) == true {
				"debug"
			} else {
				"release"
			},
			cpu_threads: candle_core::utils::get_num_threads(),
			rounds,
			chunks_per_round: chunks,
			input_bytes_per_round: bytes,
			chunk_tokens_per_round: chunkTokens,
			padded_tokens_per_round: paddedTokens,
			elapsed_seconds: elapsedSeconds,
			embeddings_per_second: chunks as f64 * roundsPerSecond,
			input_bytes_per_second: bytesPerSecond,
			chunk_tokens_per_second: chunkTokens as f64 * roundsPerSecond,
			padded_tokens_per_second: paddedTokens as f64 * roundsPerSecond,
		})
		.map_err(Into::into)
	}

	/// Runs one complete round at a time until the elapsed callback reaches the requested duration.
	/// Returns the completed round count and actual elapsed duration, or the first round error.
	#[allow(non_snake_case)]
	fn _measureRounds(
		duration: Duration,
		mut roundFn: impl FnMut() -> Result<()>,
		mut elapsedFn: impl FnMut() -> Duration,
	) -> Result<(usize, Duration)> {
		let mut rounds = 0;
		loop {
			roundFn()?;
			rounds += 1;
			let elapsed = elapsedFn();
			if elapsed >= duration {
				return Ok((rounds, elapsed));
			}
		}
	}
}

fn print_load_timings(timings: &ModelLoadTimings) {
    let stages = [
        ("Initialize backend", timings.device_initialization),
        ("Set up model repository", timings.repository_setup),
        ("Resolve/download config.json", timings.config_file_lookup),
        (
            "Resolve/download tokenizer.json",
            timings.tokenizer_file_lookup,
        ),
        (
            "Resolve/download model.safetensors",
            timings.weights_file_lookup,
        ),
        (
            "Resolve/download pooling config",
            timings.pooling_file_lookup,
        ),
        (
            "Resolve/download sentence settings",
            timings.sentence_config_file_lookup,
        ),
        ("Read and parse configuration", timings.config_parsing),
        ("Set up tokenizer", timings.tokenizer_setup),
        ("Map model weights", timings.weights_mapping),
        ("Build model", timings.model_setup),
    ];
    eprintln!("Model loading stages:");
    for (name, duration) in stages {
        eprintln!("  {name}: {:.2} seconds", duration.as_secs_f64());
    }
}

fn input_mode(requested: ChunkingMode, path: Option<&Path>) -> ChunkingMode {
    if requested != ChunkingMode::Auto {
        return requested;
    }
    let extension = path
        .and_then(Path::extension)
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => ChunkingMode::Markdown,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "c" | "h" | "cpp" | "go" | "java" | "toml"
        | "json" => ChunkingMode::Code,
        "log" | "jsonl" => ChunkingMode::Lines,
        _ => ChunkingMode::Text,
    }
}

fn read_inputs(arguments: &Arguments) -> Result<Vec<Input>> {
    let mut inputs = arguments
        .texts
        .iter()
        .enumerate()
        .map(|(index, text)| Input {
            source: format!("text:{}", index + 1),
            text: text.clone(),
            mode: input_mode(arguments.chunking, None),
        })
        .collect::<Vec<_>>();
    for path in &arguments.files {
        if arguments.verbose {
            eprintln!("Reading `{}`...", path.display());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Could not read UTF-8 text from `{}`", path.display()))?;
        if arguments.verbose {
            eprintln!("Read {} bytes from `{}`.", text.len(), path.display());
        }
        inputs.push(Input {
            source: path.display().to_string(),
            text,
            mode: input_mode(arguments.chunking, Some(path)),
        });
    }
    if !inputs.is_empty() {
        return Ok(inputs);
    }
    if std::io::stdin().is_terminal() {
        eprintln!("Paste text, then press Control-D on a new line to compute its embedding.");
    } else {
        eprintln!("Reading text from standard input.");
    }
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("Could not read text from standard input")?;
    if text.trim().is_empty() {
        bail!("No text was provided")
    }
    Ok(vec![Input {
        source: "stdin".to_string(),
        text,
        mode: input_mode(arguments.chunking, None),
    }])
}

#[cfg(test)]
#[allow(non_snake_case)]
mod benchmarkTests {
	use super::*;
	use std::cell::Cell;

	/// Checks the completed count and elapsed time with faster and slower rounds.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkCountsTheLastRound() {
		let duration = Duration::from_secs(10);
		for (roundSeconds, expectedRounds, expectedSeconds) in [(2, 5, 10), (6, 2, 12), (12, 1, 12)] {
			let clock = Cell::new(Duration::ZERO);
			let result = BenchmarkOutput::_measureRounds(
				duration,
				|| {
					clock.set(clock.get() + Duration::from_secs(roundSeconds));
					Ok(())
				},
				|| clock.get(),
			)
			.unwrap();
			assert_eq!(result, (expectedRounds, Duration::from_secs(expectedSeconds)));
		}
	}

	/// Checks the duration default and that an explicit duration requires benchmark mode.
	#[test]
	#[allow(non_snake_case)]
	fn benchmarkParsesDuration() {
		let normal = Arguments::try_parse_from(["localai_embeddings", "--text", "hello"]).unwrap();
		assert_eq!(normal.benchmark, false);
		let benchmark =
			Arguments::try_parse_from(["localai_embeddings", "--benchmark", "--text", "hello"]).unwrap();
		assert_eq!(Duration::from(benchmark.duration), Duration::from_secs(10));
		let custom = Arguments::try_parse_from(["localai_embeddings", "--benchmark", "--duration", "500ms"])
			.unwrap();
		assert_eq!(Duration::from(custom.duration), Duration::from_millis(500));
		assert!(Arguments::try_parse_from(["localai_embeddings", "--duration", "500ms"]).is_err());
		assert!(Arguments::try_parse_from(["localai_embeddings", "--benchmark", "--duration", "bogus"]).is_err());
	}
}
