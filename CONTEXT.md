# Directory Context: `/`

## Purpose
Rust libraries and command line tools that run AI models on the computer.

## Key Exports & Entry Points
- `packages/localai_embeddings/`: text embeddings — see its own CONTEXT.md.
- `packages/localai_decisions/`: the decision models based on Qwen3.5 with llama.cpp — see its own CONTEXT.md.
- `packages/localai_llm/`: a large language model that generates text with llama.cpp — see its own CONTEXT.md.
- `packages/worktree_tools/`: the commands `pnpm worktree:create`, `pnpm target:clean`, and `pnpm target:prune` — see its own CONTEXT.md.
- `Cargo.toml`: the Cargo workspace of the three Rust packages, with one `Cargo.lock` and one folder `target` at the root.
- `.agents/skills/`: the skills of the coding agent plugin for Codex, copied from [coding_agent_plugin](https://github.com/jeromeetienne/coding_agent_plugin).
- `.claude/skills/`: the same skills for Claude Code.
- `AGENTS.md`: the values of the Issue Fix Workflow of this repository.
- Command to test all the packages: `pnpm test` (fast tests), `pnpm test:full` (all the tests)
- Command to create a worktree: `pnpm worktree:create <branch>`

## Rules
- No package imports another package of this repository.
- The temporary worktrees of the Issue Fix Workflow are in `.agents/worktrees/`, which Git ignores.
- The Rust packages are one Cargo workspace, with one `Cargo.lock` and one `target` folder at the root, so a new worktree clones one target folder ([issue 14](https://github.com/heidrun-org/localai_rust/issues/14)).
- Cargo runs in the folder of a package, as the scripts of its `package.json` do, and never at the root: at the root it does not read the `.cargo/config.toml` of the package, and it builds the shared dependencies with other features, so it compiles everything again.

## Background
- The packages moved here from Heidrun Code in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954).
