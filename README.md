# localai_rust

Rust libraries and command line tools that run AI models on the computer, with no cloud service.

These packages come from the folder `packages/experiments` of [Heidrun Code](https://github.com/heidrun-org/heidrun_code). They moved to this repository in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954), with their Git history.

## Packages

- [localai_embeddings](packages/localai_embeddings/README.md): computes text embeddings.
- [localai_decisions_candle](packages/localai_decisions_candle/README.md): runs the decision model GLiNER2.5-Decide with Candle.
- [localai_decisions_llamacpp](packages/localai_decisions_llamacpp/README.md): runs the decision models based on Qwen3.5 with llama.cpp.

Each package is a separate Cargo package with its own `Cargo.lock`. No package depends on another package.

## Commands

Run each command at the root of the repository.

- `pnpm --filter <package> cli`: runs the command line tool of a package.
- `pnpm --filter <package> test`: runs the tests of a package.
- `pnpm test`: runs the tests of all the packages.

The tests of `localai_decisions_candle` and `localai_decisions_llamacpp` need large models (1.9 GB and 0.81 GB). The first run downloads them into the Hugging Face cache. The build of `localai_decisions_llamacpp` needs `cmake`.
