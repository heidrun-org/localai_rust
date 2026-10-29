# Directory Context: `/`

## Purpose
Rust libraries and command line tools that run AI models on the computer.

## Key Exports & Entry Points
- `packages/localai_embeddings/`: text embeddings — see its own CONTEXT.md.
- `packages/localai_decisions/`: the decision models based on Qwen3.5 with llama.cpp — see its own CONTEXT.md.
- `.agents/skills/`: the skills of the coding agent plugin for Codex, copied from [coding_agent_plugin](https://github.com/jeromeetienne/coding_agent_plugin).
- `.claude/skills/`: the same skills for Claude Code.
- `AGENTS.md`: the values of the Issue Fix Workflow of this repository.
- Command to test all the packages: `pnpm test` (fast tests), `pnpm test:full` (all the tests)

## Rules
- No package imports another package of this repository.
- The temporary worktrees of the Issue Fix Workflow are in `.agents/worktrees/`, which Git ignores.
- Each package keeps its own `Cargo.lock` and its own `target` folder; there is no Cargo workspace at the root.

## Background
- The packages moved here from Heidrun Code in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954).
