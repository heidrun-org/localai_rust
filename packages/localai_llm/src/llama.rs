//! A small safe layer over the C functions of llama.cpp (crate `llama-cpp-sys-2`).

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;
use std::ptr::{self, NonNull};
use std::sync::{Arc, Once};

use anyhow::{bail, Context as _, Result};
use llama_cpp_sys_2 as sys;

/// A token identifier of the vocabulary of the model.
pub type Token = i32;

static BACKEND: Once = Once::new();

unsafe extern "C" fn silent_log(_level: sys::ggml_log_level, _text: *const c_char, _user_data: *mut c_void) {}

fn init_backend(verbose: bool) {
    BACKEND.call_once(|| unsafe {
        if verbose == false {
            sys::llama_log_set(Some(silent_log), ptr::null_mut());
        }
        sys::llama_backend_init();
    });
}

/// Where the weights of the model and the computation go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    /// The processor only.
    Cpu,
    /// Every layer on the GPU that the build includes (Metal, CUDA, or Vulkan).
    Gpu,
}

/// One message of a chat, for the chat template of the model.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    /// The role: `system`, `user`, or `assistant`.
    pub role: String,
    /// The text of the message.
    pub content: String,
}

/// A loaded GGUF model.
pub struct Model {
    raw: NonNull<sys::llama_model>,
}

// SAFETY: llama.cpp allows a model to be read from several threads; the package never changes it.
unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl Model {
    /// Loads a GGUF file.
    pub fn load(path: &Path, device: Device, verbose: bool) -> Result<Self> {
        init_backend(verbose);
        let path_cstring = CString::new(path.to_string_lossy().as_bytes())?;
        // A list that holds only the end mark selects no device, so the model stays on the processor.
        let mut no_device: [sys::ggml_backend_dev_t; 1] = [ptr::null_mut()];
        let raw = unsafe {
            let mut params = sys::llama_model_default_params();
            match device {
                Device::Cpu => {
                    params.n_gpu_layers = 0;
                    params.devices = no_device.as_mut_ptr();
                }
                Device::Gpu => {
                    params.n_gpu_layers = -1;
                }
            }
            sys::llama_model_load_from_file(path_cstring.as_ptr(), params)
        };
        let raw = NonNull::new(raw).with_context(|| format!("llama.cpp could not load the model file {}", path.display()))?;
        Ok(Self { raw })
    }

    fn vocab(&self) -> *const sys::llama_vocab {
        unsafe { sys::llama_model_get_vocab(self.raw.as_ptr()) }
    }

    /// Splits a text into tokens, as `common_tokenize` of llama.cpp.
    pub fn tokenize(&self, text: &str, add_special: bool, parse_special: bool) -> Vec<Token> {
        let bytes = text.as_bytes();
        let mut tokens: Vec<Token> = vec![0; bytes.len() + 2 * usize::from(add_special) + 1];
        let mut n = unsafe {
            sys::llama_tokenize(
                self.vocab(),
                bytes.as_ptr().cast(),
                bytes.len() as i32,
                tokens.as_mut_ptr(),
                tokens.len() as i32,
                add_special,
                parse_special,
            )
        };
        if n < 0 {
            tokens.resize((-n) as usize, 0);
            n = unsafe {
                sys::llama_tokenize(
                    self.vocab(),
                    bytes.as_ptr().cast(),
                    bytes.len() as i32,
                    tokens.as_mut_ptr(),
                    tokens.len() as i32,
                    add_special,
                    parse_special,
                )
            };
        }
        tokens.truncate(n.max(0) as usize);
        tokens
    }

    /// The bytes of the text of one token. One character can be split over several tokens, so the bytes
    /// are not always valid UTF-8 alone.
    pub fn token_bytes(&self, token: Token) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![0; 64];
        let mut n = unsafe { sys::llama_token_to_piece(self.vocab(), token, buffer.as_mut_ptr().cast(), buffer.len() as i32, 0, false) };
        if n < 0 {
            buffer.resize((-n) as usize, 0);
            n = unsafe { sys::llama_token_to_piece(self.vocab(), token, buffer.as_mut_ptr().cast(), buffer.len() as i32, 0, false) };
        }
        buffer.truncate(n.max(0) as usize);
        buffer
    }

    /// True when the token ends the generation, for example the end of the assistant message.
    pub fn is_end_of_generation(&self, token: Token) -> bool {
        unsafe { sys::llama_vocab_is_eog(self.vocab(), token) }
    }

    /// The number of tokens of the vocabulary.
    pub fn n_vocab(&self) -> usize {
        unsafe { sys::llama_vocab_n_tokens(self.vocab()) as usize }
    }

    /// The default chat template of the model file, or `None` when the file has none.
    pub fn chat_template(&self) -> Option<String> {
        let template = unsafe { sys::llama_model_chat_template(self.raw.as_ptr(), ptr::null()) };
        if template.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(template) }.to_string_lossy().into_owned())
    }

    /// Writes the messages with the chat template of the model, and ends with the start of the assistant
    /// message. llama.cpp knows a fixed list of templates, and recognizes the template of the model file
    /// from its text.
    pub fn apply_chat_template(&self, messages: &[ChatMessage]) -> Result<String> {
        let template = self.chat_template().context("The model file has no chat template; use --raw")?;
        let template_cstring = CString::new(template)?;
        let mut strings = Vec::with_capacity(messages.len());
        for message in messages {
            strings.push((CString::new(message.role.as_str())?, CString::new(message.content.as_str())?));
        }
        let chat: Vec<sys::llama_chat_message> = strings
            .iter()
            .map(|(role, content)| sys::llama_chat_message {
                role: role.as_ptr(),
                content: content.as_ptr(),
            })
            .collect();
        let total: usize = messages.iter().map(|m| m.role.len() + m.content.len()).sum();
        let mut buffer: Vec<u8> = vec![0; 2 * total + 256];
        let apply = |buffer: &mut Vec<u8>| unsafe {
            sys::llama_chat_apply_template(
                template_cstring.as_ptr(),
                chat.as_ptr(),
                chat.len(),
                true,
                buffer.as_mut_ptr().cast(),
                buffer.len() as i32,
            )
        };
        let mut n = apply(&mut buffer);
        if n < 0 {
            bail!("llama.cpp does not know the chat template of the model file; use --raw");
        }
        if n as usize > buffer.len() {
            buffer.resize(n as usize, 0);
            n = apply(&mut buffer);
        }
        buffer.truncate(n as usize);
        String::from_utf8(buffer).context("The chat template wrote text that is not valid UTF-8")
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe { sys::llama_model_free(self.raw.as_ptr()) }
    }
}

/// The settings of a context.
#[derive(Clone, Copy, Debug)]
pub struct ContextSettings {
    /// The number of positions: the prompt and the generated text together.
    pub n_ctx: u32,
    /// The largest number of tokens of one evaluation; a longer prompt is evaluated in several parts.
    pub n_batch: u32,
    /// The number of processor threads.
    pub n_threads: i32,
}

/// A context: the memory of the evaluated tokens and the logits of the last evaluation.
pub struct Context {
    raw: NonNull<sys::llama_context>,
    n_batch: usize,
    _model: Arc<Model>,
}

// SAFETY: a context is used by one thread at a time; the engine keeps it behind a mutex.
unsafe impl Send for Context {}

impl Context {
    /// Creates a context with one sequence.
    pub fn new(model: Arc<Model>, settings: ContextSettings) -> Result<Self> {
        let raw = unsafe {
            let mut params = sys::llama_context_default_params();
            params.n_ctx = settings.n_ctx;
            params.n_batch = settings.n_batch;
            params.n_ubatch = settings.n_batch;
            params.n_seq_max = 1;
            params.n_threads = settings.n_threads;
            params.n_threads_batch = settings.n_threads;
            params.no_perf = true;
            sys::llama_init_from_model(model.raw.as_ptr(), params)
        };
        let raw = NonNull::new(raw).context("llama.cpp could not create a context")?;
        Ok(Self {
            raw,
            n_batch: settings.n_batch as usize,
            _model: model,
        })
    }

    /// The number of positions of the context.
    pub fn n_ctx(&self) -> usize {
        unsafe { sys::llama_n_ctx(self.raw.as_ptr()) as usize }
    }

    /// Forgets every evaluated token.
    pub fn clear(&mut self) {
        unsafe {
            let memory = sys::llama_get_memory(self.raw.as_ptr());
            if memory.is_null() == false {
                sys::llama_memory_clear(memory, true);
            }
        }
    }

    /// Evaluates tokens after the tokens already in the memory, in parts of at most the batch size. The
    /// logits of the last token are then ready for the sampler.
    pub fn decode(&mut self, tokens: &[Token]) -> Result<()> {
        for part in tokens.chunks(self.n_batch) {
            let mut part = part.to_vec();
            let result = unsafe {
                let batch = sys::llama_batch_get_one(part.as_mut_ptr(), part.len() as i32);
                sys::llama_decode(self.raw.as_ptr(), batch)
            };
            if result != 0 {
                bail!("llama.cpp could not evaluate the tokens (error {result})");
            }
        }
        Ok(())
    }

    /// Waits until the GPU has finished the last evaluation, so that a timer measures all of it.
    pub fn synchronize(&mut self) {
        unsafe { sys::llama_synchronize(self.raw.as_ptr()) }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { sys::llama_free(self.raw.as_ptr()) }
    }
}

/// The settings of the choice of the next token.
#[derive(Clone, Copy, Debug)]
pub struct SamplerSettings {
    /// 0 always takes the most probable token; a larger value gives more varied text.
    pub temperature: f32,
    /// Only the `top_k` most probable tokens can be taken; 0 turns the limit off.
    pub top_k: i32,
    /// Values above 1 make a token that is already in the last 64 tokens less probable.
    pub repeat_penalty: f32,
    /// The seed of the random choice, for a text that is the same at each run.
    pub seed: u32,
}

/// A sampler: chooses the next token from the logits of the last evaluation.
pub struct Sampler {
    raw: NonNull<sys::llama_sampler>,
}

impl Sampler {
    /// Creates the chain of samplers: repeat penalty, top k, temperature, then the choice.
    pub fn new(model: &Model, settings: SamplerSettings) -> Result<Self> {
        let raw = unsafe {
            let chain = sys::llama_sampler_chain_init(sys::llama_sampler_chain_default_params());
            if settings.repeat_penalty != 1.0 {
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_penalties(model.n_vocab() as i32, 64, settings.repeat_penalty, 0.0, 0.0));
            }
            if settings.temperature <= 0.0 {
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_greedy());
            } else {
                if settings.top_k > 0 {
                    sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_top_k(settings.top_k));
                }
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_temp(settings.temperature));
                sys::llama_sampler_chain_add(chain, sys::llama_sampler_init_dist(settings.seed));
            }
            chain
        };
        let raw = NonNull::new(raw).context("llama.cpp could not create a sampler")?;
        Ok(Self { raw })
    }

    /// Chooses the next token from the logits of the last evaluated token, and remembers it for the
    /// repeat penalty.
    pub fn sample(&mut self, context: &Context) -> Token {
        unsafe { sys::llama_sampler_sample(self.raw.as_ptr(), context.raw.as_ptr(), -1) }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe { sys::llama_sampler_free(self.raw.as_ptr()) }
    }
}

/// True when the build includes a GPU backend.
pub fn has_gpu_backend() -> bool {
    cfg!(any(feature = "metal", feature = "cuda", feature = "vulkan"))
}
