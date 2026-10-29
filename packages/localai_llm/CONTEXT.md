# Directory Context: `/packages/localai_llm`

## Purpose
This package runs a large language model on the computer, with llama.cpp, a Rust library, and a command line tool. It reads a prompt, writes it with the chat template of the model, and generates the answer one token at a time.

## Key Exports & Entry Points
- `src/`: the Rust library and the command line tool. See its own CONTEXT.md.
- `tests/`: the Rust tests of the command line tool. See its own CONTEXT.md.
- `README.md`: installation, use, the model, the speed, and the tests.
- Command to run this folder: `pnpm --filter localai_llm cli --prompt "<prompt>"`
- Command to measure speed: `pnpm --filter localai_llm cli benchmark`
- Command to test this folder: `pnpm --filter localai_llm test`

## Rules
- The model runs on the computer and the package never sends the prompt to a hosted inference service.
- The default model is LiquidAI/LFM2.5-1.2B-Instruct (file `Q4_K_M` of `LiquidAI/LFM2.5-1.2B-Instruct-GGUF`).
- The default build runs on the processor only: `.cargo/config.toml` sets `GGML_METAL=OFF`, and a GPU build needs a feature (`metal`, `cuda`, or `vulkan`); the feature `metal` also needs `GGML_METAL=ON`.
- The version of `llama-cpp-sys-2` is pinned to the version of `localai_decisions`; a new version must still run the default model.
- Cached model files are used at once by default; `--check-model-updates` asks Hugging Face for a newer file.
- The package depends on no other package of this repository: the llama.cpp layer and the Hugging Face cache code are copies of the code of `localai_decisions`.

## Background
- The package comes from https://github.com/heidrun-org/localai_rust/issues/2, which also says why LiquidAI/LFM3-1.7B is not the default model.
