# localai_rust

Rust libraries and command line tools for local AI models and embedding search.

The model packages come from the folder `packages/experiments` of [Heidrun Code](https://github.com/heidrun-org/heidrun_code). They moved to this repository in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954), with their Git history.

## Packages

- [localai_embeddings](packages/localai_embeddings/README.md): computes text embeddings.
- [localai_decisions](packages/localai_decisions/README.md): runs the decision models based on Qwen3.5 with llama.cpp.
- [localai_llm](packages/localai_llm/README.md): runs a large language model that generates text, with llama.cpp.
- [localai_rag](packages/localai_rag/README.md): searches named local document collections by meaning.
- [localai_vector_store](packages/localai_vector_store/README.md): stores and searches embeddings.

The Rust packages are one Cargo workspace, with one `Cargo.lock` and one folder `target` at the root, so they share their compiled dependencies. No package depends on another package. Run Cargo in the folder of a package, as the scripts of each `package.json` do, and not at the root of the repository.

The worktree commands use a pinned version of [Faster Rust Work Trees](https://github.com/jeromeetienne/faster_rust_worktrees). The repository settings are in [.faster_rust_worktrees.yaml](.faster_rust_worktrees.yaml).

## Local AI Rust skills

These skills teach Codex and Claude Code how to use the Local AI Rust command line tools.

- [`localai_decisions`](skills/localai_decisions/SKILL.md) answers yes-or-no or multiple-choice questions with a local model and reports the selected option and probabilities.
- [`localai_embeddings`](skills/localai_embeddings/SKILL.md) turns text into numeric vectors that represent its meaning, so you can compare texts or save the vectors for later use.
- [`localai_llm`](skills/localai_llm/SKILL.md) generates or summarizes text with a local language model.
- [`localai-rag`](skills/localai_rag/SKILL.md) searches collections of local Markdown and plain-text documents by meaning and cites the matching sources.
- [`localai_vector_store`](skills/localai_vector_store/SKILL.md) saves embeddings in local collections and searches them for similar entries.

### Install the skills

With Node.js and npm available, run this command from the root of the project where you use Codex or Claude Code. The command installs all five Local AI Rust skills for both agents.

```bash
npx --yes skills add https://github.com/heidrun-org/localai_rust/tree/main/skills \
  --skill '*' --agent codex claude-code --copy --yes
```

To install one skill, use its name from the list above. For example, this command installs the local document search skill.

```bash
npx --yes skills add heidrun-org/localai_rust \
  --skill localai-rag --agent codex --agent claude-code --copy --yes
```

The commands copy each skill's instructions into `.agents/skills/` for Codex and `.claude/skills/` for Claude Code. Start a new session in your project and ask the agent to use the skill by name. The agent reuses an available command line tool and model files. On first use, the agent asks for approval if it needs to set up the command line tool with `cgx` or download model files.

## Commands

Run each command at the root of the repository.

- `pnpm --filter <package> cli`: runs the command line tool of a package.
- `pnpm --filter <package> test`: runs the tests of a package.
- `pnpm build`: runs the build scripts of the workspace packages.
- `pnpm test`: runs the fast tests of all the packages. These tests check that the code and the model run.
- `pnpm test:full`: runs all the tests, also the slow tests that compare the answers of the model with the reference answers.
- `pnpm worktree:create <branch>` creates `.agents/worktrees/<branch>` from `HEAD` and installs the locked dependencies. Pass `--base main` to start from `main`.
- `pnpm target:clean` previews worktree build folders that have had no activity for seven days. Pass `--days <number>` to change the age.
- `pnpm target:prune` previews obsolete Cargo build files in the current checkout using the commands in the repository settings.

Run `pnpm test` in the main checkout before creating a worktree to populate the Cargo build folder. Stop the main checkout's builds before creating the worktree. On a file system that supports copy-on-write cloning, the tool reuses the available build output while keeping each worktree's files independent. Python 3 is required to preserve file times precisely.

To apply a cleanup, stop the affected builds and use `--apply`. Replace the former `--yes` option with `--apply` for both commands.

The tests of `localai_decisions` need a large model (0.81 GB). The first run downloads it into the Hugging Face cache. The build of `localai_decisions` needs `cmake`.

The slow tests of `localai_llm` need the model LiquidAI/LFM2.5-1.2B-Instruct (0.73 GB), which the first run downloads into the Hugging Face cache. The build of `localai_llm` needs `cmake`.
