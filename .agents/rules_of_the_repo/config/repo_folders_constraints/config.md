# Repo Folders Constraints

## Packages

| Folder | Does | Never does | May import | Language and technologies | Workflow |
|---|---|---|---|---|---|
| `packages/localai_decisions` | Runs local model decisions over supplied choices | Manages Git worktrees | no other package | Rust and Cargo, with a C++ bridge to llama.cpp | Run the package tests after changes |
| `packages/localai_embeddings` | Produces text embeddings locally | Manages Git worktrees | no other package | Rust and Cargo | Run the package tests after changes |
| `packages/localai_llm` | Runs local language model requests | Manages Git worktrees | no other package | Rust and Cargo | Run the package tests after changes |
| `packages/localai_vector_store` | Stores local embeddings and searches them by similarity | Runs embedding models or manages Git worktrees | no other package | Rust, Cargo, and SQLite | Run the package tests after changes |
| `packages/localai_rag` | Indexes, refreshes, and searches named collections of local Markdown and plain-text files | Manages Git worktrees | `localai_embeddings` and `localai_vector_store` | Rust and Cargo | Run the package tests after changes |

## Other folders

| Folder | Does | Never does | Language and technologies | Workflow |
|---|---|---|---|---|
| `/` | Holds workspace manifests, shared commands, and repository instructions | Holds package implementation | Cargo and pnpm workspace files, Markdown | Keep shared commands and their documentation aligned |
| `packages/` | Groups workspace packages | Holds configuration for installed skills | Cargo and pnpm workspaces | Add or remove packages with their workspace and lockfile changes |
| `.agents/` | Holds installed skills, repository rules, and temporary worktrees | Holds product source outside temporary worktrees | Markdown and skill resources | Use the configured worktree command and review changes to repository rules |
| `.claude/skills/` | Exposes installed skills to Claude Code | Holds package implementation | Skill links | Update with the corresponding installed skills |
| `docs/` | Holds project documentation when added | Holds package implementation | Markdown | Follow the docs folder system |
| `skills/` | Holds distributable Local AI skills | Holds package implementation | Markdown and skill resources | Keep each skill self-contained and test it in both agents |
| `testbed_folders/` | Holds the shared Codex and Claude Code test workspace | Holds downloaded models or compiled tools | Markdown and symbolic links | Generate the local skill copies from `skills/` |
| `scripts/` | Holds shared repository maintenance and test scripts | Holds package implementation | Shell scripts | Document each command at its point of use |
