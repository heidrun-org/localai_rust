# Repo Folders Constraints

## Packages

| Folder | Does | Never does | May import | Language and technologies | Workflow |
|---|---|---|---|---|---|
| `packages/localai_decisions` | Runs local model decisions over supplied choices | Manages Git worktrees | no other package | Rust and Cargo, with a C++ bridge to llama.cpp | Run the package tests after changes |
| `packages/localai_embeddings` | Produces text embeddings locally | Manages Git worktrees | no other package | Rust and Cargo | Run the package tests after changes |
| `packages/localai_llm` | Runs local language model requests | Manages Git worktrees | no other package | Rust and Cargo | Run the package tests after changes |

## Other folders

| Folder | Does | Never does | Language and technologies | Workflow |
|---|---|---|---|---|
| `/` | Holds workspace manifests, shared commands, and repository instructions | Holds package implementation | Cargo and pnpm workspace files, Markdown | Keep shared commands and their documentation aligned |
| `packages/` | Groups workspace packages | Holds configuration for installed skills | Cargo and pnpm workspaces | Add or remove packages with their workspace and lockfile changes |
| `.agents/` | Holds installed skills, repository rules, and temporary worktrees | Holds product source outside temporary worktrees | Markdown and skill resources | Use the configured worktree command and review changes to repository rules |
| `.claude/skills/` | Exposes installed skills to Claude Code | Holds package implementation | Skill links | Update with the corresponding installed skills |
| `docs/` | Holds project documentation when added | Holds package implementation | Markdown | Follow the docs folder system |
