# Directory Context: `/`

## Purpose
Rust libraries and command line tools that run AI models on the computer.

## Key Exports & Entry Points
- `packages/localai_embeddings/`: text embeddings — see its own CONTEXT.md.
- `packages/localai_decisions_candle/`: the decision model GLiNER2.5-Decide with Candle — see its own CONTEXT.md.
- `packages/localai_decisions_llamacpp/`: the decision models based on Qwen3.5 with llama.cpp — see its own CONTEXT.md.
- Command to test all the packages: `pnpm test`

## Rules
- No package imports another package of this repository.
- Each package keeps its own `Cargo.lock` and its own `target` folder; there is no Cargo workspace at the root.

## Background
- The packages moved here from Heidrun Code in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954).
