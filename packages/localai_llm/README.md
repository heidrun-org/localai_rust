# Local AI large language model

`localai_llm` runs a large language model on your computer, with llama.cpp, a Rust library, and a command line tool. The tool reads a prompt, writes it with the chat template of the model, and writes the text that the model generates. After the first download of the model file, the prompt and the text never leave your computer.

## Requirements

- Rust, Cargo, and CMake. The first build compiles llama.cpp, which takes about two minutes.
- Internet access on the first run, to download the model file.
- For the default model: about 1 GB of free disk space and about 2 GB of free memory.

## Model

The model is a parameter, `--model`. The default is [LiquidAI/LFM2.5-1.2B-Instruct](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct), file `LFM2.5-1.2B-Instruct-Q4_K_M.gguf` (0.73 GB) of the repository `LiquidAI/LFM2.5-1.2B-Instruct-GGUF`. Its license is the LFM Open License v1.0 of LiquidAI.

- `--model LiquidAI/LFM2.5-1.2B-Instruct-GGUF:Q8_0` selects another quantization. A repository `owner/Name-GGUF` with the quantization `Q` gives the file `Name-Q.gguf`.
- `--model path/to/file.gguf` reads a local file.
- `--check-model-updates` asks Hugging Face for a newer model file. By default, a cached file is used at once.

Another model must have an architecture that llama.cpp v0.6.0 knows. LiquidAI/LFM3-1.7B does not run: its architecture `lfm3` is in llama.cpp only from release `b11938`, and no version of `llama-cpp-sys-2` contains that release yet.

## Generate text

```bash
pnpm --filter localai_llm cli --prompt "What is the capital of France?"
echo "Write a haiku about the sea." | pnpm --filter localai_llm cli
pnpm --filter localai_llm cli --system "Answer in French." --prompt "Name three colors."
pnpm --silent --filter localai_llm cli --prompt "Say hello." --json
```

The text appears while the model writes it. With `--json`, the tool writes the result at the end: `model`, `text`, `prompt_tokens`, `generated_tokens`, `stop_reason` (`end`, `max_tokens`, or `context_full`), `prompt_seconds`, and `generation_seconds`.

Other options:

- `--max-tokens 512` sets the largest number of generated tokens (512 by default).
- `--temperature 0.1`, `--top-k 50`, and `--repeat-penalty 1.05` set the choice of the next token. The defaults are the values of the model card of LFM2.5. `--temperature 0` always takes the most probable token.
- `--seed 0` sets the seed of the random choice (0 by default), so that two runs with the same options give the same text.
- `--raw` gives the prompt to the model as it is, without the chat template.
- `--context-size 4096` sets the number of positions of the prompt and the generated text together (4096 by default). A larger value takes more memory.
- `--threads 8` sets the number of processor threads. The default is the number of performance cores, at most 8.
- `--output answer.txt` writes the result in a file, and `--verbose` prints the logs of llama.cpp, the loading time, and the speed.

The package command runs inside the package folder, so a relative file path starts there. Use an absolute path for a file elsewhere.

## Use the Rust library

```rust
use localai_llm::{GenerationRequest, LlmConfig, LlmEngine};

let engine = LlmEngine::load(LlmConfig::default())?;
let generation = engine.generate(&GenerationRequest::new("What is the capital of France?"), |part| print!("{part}"))?;
println!("\n{} tokens", generation.generated_tokens);
```

`LlmEngine::load` reads the model file and takes about one second. Keep the engine and call `generate` for each prompt. The callback receives each new part of the text.

## GPU

The default build runs on the processor only. A build option adds the GPU, and `--backend gpu` selects it. The spelling `--device gpu` works too:

```bash
cd packages/localai_llm
GGML_METAL=ON cargo run --release --features metal -- --backend gpu --prompt "What is the capital of France?"
```

On macOS, llama.cpp builds Metal by default, so the file `.cargo/config.toml` sets `GGML_METAL=OFF` and the Metal build sets `GGML_METAL=ON`. The features `cuda` and `vulkan` select an NVIDIA graphics card or Vulkan in the same way; they were not tested. A GPU in a build without a GPU feature gives an error, and the tool never switches to another device by itself. Changing between the two builds compiles llama.cpp again; use another `CARGO_TARGET_DIR` to keep both.

## Measure speed

```bash
pnpm --filter localai_llm cli benchmark
pnpm --filter localai_llm cli benchmark --generations 5 --max-tokens 256
pnpm --filter localai_llm cli benchmark --output /tmp/localai_llm-speed.json
pnpm --silent --filter localai_llm cli benchmark --json
GGML_METAL=ON cargo run --release --features metal -- benchmark --backend gpu
```

The command measures two fixed prompts: a short prompt (16 tokens) and a long prompt (515 tokens). The model loads once, then each case completes a warmup before timing starts. Each case then completes 3 measured generations of at most 128 tokens. The benchmark always takes the most probable token, so each run generates the same text.

The table shows, for one generation, the prompt tokens, the generated tokens, and the seconds, then the prompt tokens per second (the evaluation of the prompt) and the generated tokens per second (the generation after the prompt). Peak process resident memory is the high-water mark of the operating system for this process, not a measurement of separate GPU allocations.

## Speed

Time on an Apple M1 Pro with the default model, after the model is loaded:

| Case | Processor, 8 threads | Metal |
| --- | --- | --- |
| Short prompt: prompt tokens per second | 237 | 706 |
| Short prompt: generated tokens per second | 53 | 118 |
| Long prompt: prompt tokens per second | 251 | 1278 |
| Long prompt: generated tokens per second | 30 | 118 |
| Peak process resident memory | 1520 MiB | 836 MiB |

## Tests

```bash
pnpm --filter localai_llm test
pnpm --filter localai_llm test:full
```

The command `test` runs the fast tests in a few seconds: they check the options and the errors, and they do not load the model. The command `test:full` also runs the slow tests, marked with `#[ignore]`, that generate text with the model. The tests use a release build, because the model is too slow in a debug build. The slow tests need the default model in the Hugging Face cache; the first run downloads it (0.73 GB).
