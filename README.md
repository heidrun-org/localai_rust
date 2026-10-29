# localai_rust

Rust libraries and command line tools that run AI models on the computer, with no cloud service.

These packages come from the folder `packages/experiments` of [Heidrun Code](https://github.com/heidrun-org/heidrun_code). They moved to this repository in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954), with their Git history.

## Packages

- [localai_embeddings](packages/localai_embeddings/README.md): computes text embeddings.
- [localai_decisions](packages/localai_decisions/README.md): runs the decision models based on Qwen3.5 with llama.cpp.
- [localai_llm](packages/localai_llm/README.md): runs a large language model that generates text, with llama.cpp.

Each package is a separate Cargo package with its own `Cargo.lock`. No package depends on another package.

## Skills of the coding agent

The folders `.agents/skills/` (for Codex) and `.claude/skills/` (for Claude Code) hold the skills of the coding agent plugin [coding_agent_plugin](https://github.com/jeromeetienne/coding_agent_plugin), copied with this command:

```bash
npx skills add jeromeetienne/coding_agent_plugin --skill '*' --agent claude-code --agent codex --copy --yes
```

The file [AGENTS.md](AGENTS.md) gives the values of the Issue Fix Workflow of this repository.

## Commands

Run each command at the root of the repository.

- `pnpm --filter <package> cli`: runs the command line tool of a package.
- `pnpm --filter <package> test`: runs the tests of a package.
- `pnpm test`: runs the fast tests of all the packages. These tests check that the code and the model run.
- `pnpm test:full`: runs all the tests, also the slow tests that compare the answers of the model with the reference answers.

The tests of `localai_decisions` need a large model (0.81 GB). The first run downloads it into the Hugging Face cache. The build of `localai_decisions` needs `cmake`.

The slow tests of `localai_llm` need the model LiquidAI/LFM2.5-1.2B-Instruct (0.73 GB), which the first run downloads into the Hugging Face cache. The build of `localai_llm` needs `cmake`.
