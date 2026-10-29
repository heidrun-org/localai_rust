//! Runs a large language model on the computer, with llama.cpp.
//!
//! The engine reads a prompt, writes it with the chat template of the model, and generates the answer one
//! token at a time. The default model is LiquidAI/LFM2.5-1.2B-Instruct.

pub mod llama;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{bail, Context as _, Result};
use hf_hub::{HFClientSync, HFError};
use serde::Serialize;

use crate::llama::{ChatMessage, Context, ContextSettings, Model, Sampler, Token};

pub use crate::llama::{Device, SamplerSettings};

/// The model that the package uses when the caller names none.
pub const DEFAULT_MODEL: &str = "LiquidAI/LFM2.5-1.2B-Instruct-GGUF";

/// The quantization that the package uses when the model name gives none.
pub const DEFAULT_QUANTIZATION: &str = "Q4_K_M";

/// The settings of the choice of the next token that the model card of LFM2.5 recommends.
pub const DEFAULT_SAMPLER: SamplerSettings = SamplerSettings {
    temperature: 0.1,
    top_k: 50,
    repeat_penalty: 1.05,
    seed: 0,
};

/// The settings of an engine.
#[derive(Clone, Debug)]
pub struct LlmConfig {
    /// A Hugging Face repository with an optional quantization, for example
    /// `LiquidAI/LFM2.5-1.2B-Instruct-GGUF:Q8_0`, or the path of a local `.gguf` file.
    pub model: String,
    /// Where the model runs.
    pub device: Device,
    /// The number of processor threads; `None` uses the number of performance cores, at most 8.
    pub threads: Option<i32>,
    /// The number of positions: the prompt and the generated text together.
    pub context_size: u32,
    /// The largest number of tokens of one evaluation.
    pub batch_size: u32,
    /// True to ask Hugging Face for a newer model file, even when the cache has one.
    pub check_model_updates: bool,
    /// True to print the logs of llama.cpp on standard error.
    pub verbose: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            device: Device::Cpu,
            threads: None,
            context_size: 4096,
            batch_size: 2048,
            check_model_updates: false,
            verbose: false,
        }
    }
}

/// What to generate.
#[derive(Clone, Debug)]
pub struct GenerationRequest {
    /// The text of the user.
    pub prompt: String,
    /// An optional system message, before the text of the user.
    pub system: Option<String>,
    /// False to give the prompt to the model as it is, without the chat template.
    pub use_chat_template: bool,
    /// The largest number of generated tokens.
    pub max_tokens: usize,
    /// The choice of the next token.
    pub sampler: SamplerSettings,
}

impl GenerationRequest {
    /// A request with the chat template, 512 tokens at most, and the default sampler.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            system: None,
            use_chat_template: true,
            max_tokens: 512,
            sampler: DEFAULT_SAMPLER,
        }
    }
}

/// Why the generation stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model wrote its end token.
    End,
    /// The generation reached the largest number of tokens of the request.
    MaxTokens,
    /// The context has no free position.
    ContextFull,
}

/// The result of a generation.
#[derive(Clone, Debug, Serialize)]
pub struct Generation {
    /// The generated text, without the end token.
    pub text: String,
    /// The number of tokens of the prompt, after the chat template.
    pub prompt_tokens: usize,
    /// The number of generated tokens, without the end token.
    pub generated_tokens: usize,
    /// Why the generation stopped.
    pub stop_reason: StopReason,
    /// The time of the evaluation of the prompt.
    pub prompt_seconds: f64,
    /// The time of the generation of the tokens, after the prompt.
    pub generation_seconds: f64,
}

/// A loaded model, ready to generate text.
pub struct LlmEngine {
    model_path: PathBuf,
    model: Arc<Model>,
    context: Mutex<Context>,
}

impl LlmEngine {
    /// Finds the model file (in the Hugging Face cache, or downloads it), and loads it.
    pub fn load(config: LlmConfig) -> Result<Self> {
        if config.device == Device::Gpu && llama::has_gpu_backend() == false {
            bail!("This build has no GPU backend; build with the feature metal, cuda, or vulkan (see README.md)");
        }
        if config.batch_size == 0 {
            bail!("The batch size must be at least 1");
        }
        if config.context_size == 0 {
            bail!("The context size must be at least 1");
        }
        let model_path = resolve_model_file(&config.model, config.check_model_updates)?;
        let model = Arc::new(Model::load(&model_path, config.device, config.verbose)?);
        let settings = ContextSettings {
            n_ctx: config.context_size,
            n_batch: config.batch_size.min(config.context_size),
            n_threads: config.threads.unwrap_or_else(default_threads),
        };
        let context = Context::new(model.clone(), settings)?;
        Ok(Self {
            model_path,
            model,
            context: Mutex::new(context),
        })
    }

    /// The path of the loaded model file.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// The text that the model reads: the prompt written with the chat template, or the prompt as it is.
    pub fn prompt_text(&self, request: &GenerationRequest) -> Result<String> {
        if request.use_chat_template == false {
            return Ok(request.prompt.clone());
        }
        let mut messages = Vec::new();
        if let Some(system) = &request.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        self.model.apply_chat_template(&messages)
    }

    /// Generates the answer to a request. `on_text` receives each new part of the text as soon as it is
    /// complete UTF-8, so that a caller can show the answer while it grows.
    pub fn generate(&self, request: &GenerationRequest, mut on_text: impl FnMut(&str)) -> Result<Generation> {
        let prompt = self.prompt_text(request)?;
        let prompt_tokens = self.model.tokenize(&prompt, true, true);
        if prompt_tokens.is_empty() {
            bail!("The prompt is empty");
        }

        let mut context = self.context.lock().map_err(|_| anyhow::anyhow!("The llama.cpp context is poisoned"))?;
        if prompt_tokens.len() >= context.n_ctx() {
            bail!(
                "The prompt has {} tokens, and the context has only {} positions; give a larger --context-size",
                prompt_tokens.len(),
                context.n_ctx()
            );
        }
        let mut sampler = Sampler::new(&self.model, request.sampler)?;
        context.clear();

        let started = Instant::now();
        context.decode(&prompt_tokens)?;
        context.synchronize();
        let prompt_seconds = started.elapsed().as_secs_f64();

        let started = Instant::now();
        let mut decoder = Utf8Decoder::default();
        let mut text = String::new();
        let mut generated_tokens = 0;
        let stop_reason = loop {
            if generated_tokens >= request.max_tokens {
                break StopReason::MaxTokens;
            }
            let token: Token = sampler.sample(&context);
            if self.model.is_end_of_generation(token) {
                break StopReason::End;
            }
            generated_tokens += 1;
            let part = decoder.push(&self.model.token_bytes(token));
            if part.is_empty() == false {
                on_text(&part);
                text.push_str(&part);
            }
            if prompt_tokens.len() + generated_tokens >= context.n_ctx() {
                break StopReason::ContextFull;
            }
            context.decode(&[token])?;
        };
        let rest = decoder.finish();
        if rest.is_empty() == false {
            on_text(&rest);
            text.push_str(&rest);
        }

        Ok(Generation {
            text,
            prompt_tokens: prompt_tokens.len(),
            generated_tokens,
            stop_reason,
            prompt_seconds,
            generation_seconds: started.elapsed().as_secs_f64(),
        })
    }
}

/// Joins the bytes of the tokens into text. One character can be split over several tokens, so the bytes
/// of an incomplete character wait for the next token.
#[derive(Debug, Default)]
pub struct Utf8Decoder {
    pending: Vec<u8>,
}

impl Utf8Decoder {
    /// Adds the bytes of one token, and returns the text that is now complete.
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut text = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(valid) => {
                    text.push_str(valid);
                    self.pending.clear();
                    return text;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    text.push_str(std::str::from_utf8(&self.pending[..valid]).unwrap_or_default());
                    match error.error_len() {
                        // The end is the start of a character: wait for the next token.
                        None => {
                            self.pending.drain(..valid);
                            return text;
                        }
                        Some(len) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            self.pending.drain(..valid + len);
                        }
                    }
                }
            }
        }
    }

    /// Returns the bytes that still wait, as text with replacement characters.
    pub fn finish(&mut self) -> String {
        let text = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        text
    }
}

/// The default number of processor threads: all threads on the efficiency cores of an Apple processor
/// slow the evaluation down, so the default is the number of performance cores, at most 8.
pub fn default_threads() -> i32 {
    let all = std::thread::available_parallelism().map_or(4, |n| n.get() as i32);
    let performance = std::process::Command::new("sysctl")
        .args(["-n", "hw.perflevel0.physicalcpu"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.trim().parse::<i32>().ok())
        .unwrap_or(all);
    performance.clamp(1, 8).min(all)
}

/// Finds the GGUF file of a model: a local path, or a file of a Hugging Face repository.
///
/// A repository name `owner/Name-GGUF` with the quantization `Q` gives the file `Name-Q.gguf`, as the
/// repositories of LiquidAI and ggml-org name their files. The cache is used first, without a network
/// request.
pub fn resolve_model_file(model: &str, check_model_updates: bool) -> Result<PathBuf> {
    let path = Path::new(model);
    if model.ends_with(".gguf") {
        if path.is_file() == false {
            bail!("The model file {model} does not exist");
        }
        return Ok(path.to_path_buf());
    }

    let (repository, quantization) = model.split_once(':').unwrap_or((model, DEFAULT_QUANTIZATION));
    let Some((owner, name)) = repository.split_once('/') else {
        bail!("The model must be a `.gguf` file or a Hugging Face repository of the form `owner/name`, for example {DEFAULT_MODEL}");
    };
    if owner.is_empty() || name.is_empty() || name.contains('/') || quantization.is_empty() {
        bail!("The model must be a `.gguf` file or a Hugging Face repository of the form `owner/name[:quantization]`");
    }
    let base = name.strip_suffix("-GGUF").unwrap_or(name);
    let filename = format!("{base}-{quantization}.gguf");

    let client = HFClientSync::new().context("Could not initialize the Hugging Face model cache")?;
    let repo = client.model(owner.to_string(), name.to_string());
    if check_model_updates == false {
        match repo
            .download_file()
            .filename(filename.clone())
            .revision("main".to_string())
            .local_files_only(true)
            .send()
        {
            Ok(path) => return Ok(path),
            Err(HFError::LocalEntryNotFound { .. }) => {}
            Err(error) => return Err(error).with_context(|| format!("Could not read the cached model file `{filename}`")),
        }
    }
    repo.download_file()
        .filename(filename.clone())
        .revision("main".to_string())
        .send()
        .with_context(|| format!("Could not download `{filename}` from the repository {repository}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_decoder_waits_for_the_rest_of_a_split_character() {
        let mut decoder = Utf8Decoder::default();
        let bytes = "é!".as_bytes();
        assert_eq!(decoder.push(&bytes[..1]), "");
        assert_eq!(decoder.push(&bytes[1..]), "é!");
        assert_eq!(decoder.finish(), "");
    }

    #[test]
    fn the_decoder_replaces_invalid_bytes() {
        let mut decoder = Utf8Decoder::default();
        assert_eq!(decoder.push(&[b'a', 0xff, b'b']), "a\u{fffd}b");
        assert_eq!(decoder.push(&[0xc3]), "");
        assert_eq!(decoder.finish(), "\u{fffd}");
    }
}
