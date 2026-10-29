# Directory Context: `/packages/localai_llm/src`

## Purpose
This folder holds the generation library and the command line tool.

## Key Exports & Entry Points
- `lib.rs`: the public library interface: `LlmEngine` (`load`, `generate`, `prompt_text`), `LlmConfig`, `GenerationRequest`, `Generation`, `StopReason`, `SamplerSettings`, `Device`, `Utf8Decoder`, `resolve_model_file`, `DEFAULT_MODEL`, `DEFAULT_SAMPLER`.
- `llama.rs`: the small safe layer over the C functions of `llama-cpp-sys-2`: the model, the chat template, the context, and the sampler.
- `main.rs`: the command line tool, which reads a prompt from `--prompt` or from standard input, and writes the text while it grows, or JSON with `--json`.
- `benchmark.rs`: the command line subcommand `benchmark`, with a short and a long prompt, warmup, prompt tokens per second, generated tokens per second, and peak process resident memory.

## Rules
- Only `llama.rs` calls the C functions of llama.cpp; the other files use its safe types.
- The chat template comes from the model file, through `llama_chat_apply_template`, which knows a fixed list of templates; the package does not render Jinja templates.
- The timer of the prompt evaluation stops after `llama_synchronize`, because a GPU evaluation continues after `llama_decode` returns.
- The command line accepts `--backend cpu|gpu` as a visible alias for `--device cpu|gpu`, including the benchmark subcommand.
- Benchmark timers exclude model loading and warmup, the benchmark always takes the most probable token, and it never replaces the selected backend.
