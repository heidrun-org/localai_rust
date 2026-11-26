# Directory Context: `/`

## Purpose
Rust libraries and command line tools that run AI models and search saved embeddings on the computer.

## Key Exports & Entry Points
- `packages/localai_embeddings/`: text embeddings — see its own CONTEXT.md.
- `packages/localai_decisions/`: the decision models based on Qwen3.5 with llama.cpp — see its own CONTEXT.md.
- `packages/localai_llm/`: a large language model that generates text with llama.cpp — see its own CONTEXT.md.
- `packages/localai_rag/`: local document collection search. See its CONTEXT.md.
- `packages/localai_vector_store/`: local embedding storage and search. See its CONTEXT.md.
- `.faster_rust_worktrees.yaml`: configures the shared Faster Rust Work Trees tool for the Cargo workspace. See the worktree commands in README.md.
- `Cargo.toml`: the Cargo workspace of the Rust packages, with one `Cargo.lock` and one folder `target` at the root.
- `.agents/skills/`: the skills of the coding agent plugin for Codex, copied from [coding_agent_plugin](https://github.com/jeromeetienne/coding_agent_plugin).
- `.claude/skills/`: the same skills for Claude Code.
- `skills/`: distributable Local AI skills. See [CONTEXT.md](skills/CONTEXT.md).
- `scripts/`: shared repository maintenance and test commands. See [CONTEXT.md](scripts/CONTEXT.md).
- `testbed_folders/`: shared standalone test workspace for the Local AI skills. See [CONTEXT.md](testbed_folders/CONTEXT.md).
- `AGENTS.md`: the values of the Issue Fix Workflow of this repository.
- Command to build all the packages: `pnpm build` (see README.md).
- Command to test all the packages: `pnpm test` (fast tests), `pnpm test:full` (all the tests)
- Command to create a worktree: `pnpm worktree:create <branch>`

## Rules
- Workspace package dependencies follow the [Repo Folders Constraints](.agents/rules_of_the_repo/config/repo_folders_constraints/config.md).
- The temporary worktrees of the Issue Fix Workflow are in `.agents/worktrees/`, which Git ignores.
- The Rust packages are one Cargo workspace, with one `Cargo.lock` and one `target` folder at the root, so a new worktree clones one target folder ([issue 14](https://github.com/heidrun-org/localai_rust/issues/14)).
- Cargo runs in the folder of a package, as the scripts of its `package.json` do, and never at the root: at the root it does not read the `.cargo/config.toml` of the package, and it builds the shared dependencies with other features, so it compiles everything again.

## Background
- The packages moved here from Heidrun Code in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954).
