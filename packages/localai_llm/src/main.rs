//! The command line tool: reads a prompt and writes the text that the model generates.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context as _, Result};
use clap::{Parser, Subcommand, ValueEnum};
use localai_llm::{Device, Generation, GenerationRequest, LlmConfig, LlmEngine, SamplerSettings, DEFAULT_MODEL, DEFAULT_SAMPLER};
use serde::Serialize;

/// Generates text with a large language model, on this computer.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Arguments {
    #[command(subcommand)]
    command: Option<Command>,

    /// A Hugging Face repository, with an optional quantization after a colon, or a local `.gguf` file.
    #[arg(long, global = true, default_value = DEFAULT_MODEL)]
    model: String,

    /// The prompt. Without it, the prompt is read from standard input.
    #[arg(long)]
    prompt: Option<String>,

    /// A system message, before the prompt.
    #[arg(long, conflicts_with = "raw")]
    system: Option<String>,

    /// Give the prompt to the model as it is, without the chat template of the model.
    #[arg(long)]
    raw: bool,

    /// The largest number of generated tokens.
    #[arg(long, default_value_t = 512)]
    max_tokens: usize,

    /// 0 always takes the most probable token; a larger value gives more varied text.
    #[arg(long, default_value_t = DEFAULT_SAMPLER.temperature)]
    temperature: f32,

    /// Only the most probable tokens can be taken; 0 turns the limit off.
    #[arg(long, default_value_t = DEFAULT_SAMPLER.top_k)]
    top_k: i32,

    /// Values above 1 make a token that is already in the last 64 tokens less probable.
    #[arg(long, default_value_t = DEFAULT_SAMPLER.repeat_penalty)]
    repeat_penalty: f32,

    /// The seed of the random choice of the tokens.
    #[arg(long, default_value_t = DEFAULT_SAMPLER.seed)]
    seed: u32,

    /// Write the text and the token counts as JSON, after the generation, instead of the text while it grows.
    #[arg(long)]
    json: bool,

    /// Where the model runs: `gpu` needs a build with the feature metal, cuda, or vulkan.
    #[arg(long, visible_alias = "backend", global = true, value_enum, default_value_t = DeviceArgument::Cpu)]
    device: DeviceArgument,

    /// The number of processor threads; by default, the number of performance cores, at most 8.
    #[arg(long, global = true)]
    threads: Option<i32>,

    /// The number of positions: the prompt and the generated text together.
    #[arg(long, global = true, default_value_t = 4096)]
    context_size: u32,

    /// The largest number of tokens of one evaluation; a longer prompt is evaluated in several parts.
    #[arg(long, global = true, default_value_t = 2048)]
    batch_size: u32,

    /// Ask Hugging Face for a newer model file, even when the cache has one.
    #[arg(long, global = true)]
    check_model_updates: bool,

    /// Write the result in this file instead of standard output.
    #[arg(long, global = true)]
    output: Option<PathBuf>,

    /// Print the logs of llama.cpp, the loading time, and the speed on standard error.
    #[arg(long, global = true)]
    verbose: bool,
}

/// The output of --json: the model name, then the fields of the generation.
#[derive(Serialize)]
struct JsonResult<'a> {
    model: &'a str,
    #[serde(flatten)]
    generation: &'a Generation,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DeviceArgument {
    Cpu,
    Gpu,
}

mod benchmark;

#[derive(Debug, Subcommand)]
enum Command {
    /// Measure the speed of the prompt evaluation and of the generation on built-in prompts.
    Benchmark(benchmark::Arguments),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut arguments = Arguments::parse();
    let config = LlmConfig {
        model: arguments.model.clone(),
        device: match arguments.device {
            DeviceArgument::Cpu => Device::Cpu,
            DeviceArgument::Gpu => Device::Gpu,
        },
        threads: arguments.threads,
        context_size: arguments.context_size,
        batch_size: arguments.batch_size,
        check_model_updates: arguments.check_model_updates,
        verbose: arguments.verbose,
    };
    if let Some(Command::Benchmark(benchmark)) = arguments.command.take() {
        return benchmark::run(benchmark, config, arguments.output);
    }
    let request = GenerationRequest {
        prompt: read_prompt(&arguments)?,
        system: arguments.system.clone(),
        use_chat_template: arguments.raw == false,
        max_tokens: arguments.max_tokens,
        sampler: SamplerSettings {
            temperature: arguments.temperature,
            top_k: arguments.top_k,
            repeat_penalty: arguments.repeat_penalty,
            seed: arguments.seed,
        },
    };

    let started = Instant::now();
    let engine = LlmEngine::load(config)?;
    if arguments.verbose {
        eprintln!("Loaded {} in {} ms", engine.model_path().display(), started.elapsed().as_millis());
    }

    // The text appears while it grows only on standard output, without --json and without --output.
    let streams = arguments.json == false && arguments.output.is_none();
    let mut stdout = std::io::stdout();
    let generation = engine.generate(&request, |part| {
        if streams {
            print!("{part}");
            let _ = stdout.flush();
        }
    })?;
    if arguments.verbose {
        eprintln!(
            "\nPrompt: {} tokens in {:.2} seconds; generation: {} tokens in {:.2} seconds; stop: {:?}",
            generation.prompt_tokens,
            generation.prompt_seconds,
            generation.generated_tokens,
            generation.generation_seconds,
            generation.stop_reason
        );
    }

    let text = if arguments.json {
        let result = JsonResult {
            model: &arguments.model,
            generation: &generation,
        };
        serde_json::to_string_pretty(&result)? + "\n"
    } else {
        generation.text.clone() + "\n"
    };
    match &arguments.output {
        Some(path) => std::fs::write(path, text).with_context(|| format!("Could not write {}", path.display()))?,
        None if streams => println!(),
        None => print!("{text}"),
    }
    Ok(())
}

fn read_prompt(arguments: &Arguments) -> Result<String> {
    if let Some(prompt) = &arguments.prompt {
        if prompt.trim().is_empty() {
            bail!("The prompt is empty");
        }
        return Ok(prompt.clone());
    }
    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content).context("Could not read standard input")?;
    if content.trim().is_empty() {
        bail!("No prompt: give --prompt, or a prompt on standard input");
    }
    Ok(content)
}
