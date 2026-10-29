# localai_rust

Rust libraries and command line tools that run AI models on the computer, with no cloud service.

These packages come from the folder `packages/experiments` of [Heidrun Code](https://github.com/heidrun-org/heidrun_code). They moved to this repository in [issue 954](https://github.com/heidrun-org/heidrun_code/issues/954), with their Git history.

## Packages

- [localai_embeddings](packages/localai_embeddings/README.md): computes text embeddings.
- [localai_decisions](packages/localai_decisions/README.md): runs the decision models based on Qwen3.5 with llama.cpp.
- [localai_llm](packages/localai_llm/README.md): runs a large language model that generates text, with llama.cpp.

The three Rust packages are one Cargo workspace, with one `Cargo.lock` and one folder `target` at the root, so they share their compiled dependencies. No package depends on another package. Run Cargo in the folder of a package, as the scripts of each `package.json` do, and not at the root of the repository.

The package [worktree_tools](packages/worktree_tools/README.md) creates a new worktree with the compiled files of the main checkout, so its first `pnpm test` compiles nothing that did not change.

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
- `pnpm worktree:create <branch>`: creates the worktree `.agents/worktrees/<branch>` on a new branch, with a copy-on-write clone of the folder `target` of the main checkout.
- `pnpm target:clean`: lists, and with `--yes` deletes, the folders `target` of the worktrees that no build used for 7 days.
- `pnpm target:prune`: lists, and with `--yes` deletes, the files of the folder `target` that the current builds do not use.

The tests of `localai_decisions` need a large model (0.81 GB). The first run downloads it into the Hugging Face cache. The build of `localai_decisions` needs `cmake`.

The slow tests of `localai_llm` need the model LiquidAI/LFM2.5-1.2B-Instruct (0.73 GB), which the first run downloads into the Hugging Face cache. The build of `localai_llm` needs `cmake`.
