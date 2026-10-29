# Directory Context: `/packages/worktree_tools`

## Purpose
A TypeScript package with three command line tools that make a new worktree fast and small on the disk: create a worktree with the compiled Rust files of the main checkout, delete the target folders of old worktrees, and delete the obsolete files of a target folder. The [README.md](README.md) gives the commands, the options, and the measures.

## Key Exports & Entry Points
- `src/index.ts`: the public interface for another package. It exports the classes `WorktreeCreate`, `TargetClean`, `TargetPrune`, `DirectoryClone`, `SourceDates`, and `BuildScriptOutputs` and their types.
- `src/worktree_create_cli.ts`: `pnpm worktree:create <name>`.
- `src/target_clean_cli.ts`: `pnpm target:clean`.
- `src/target_prune_cli.ts`: `pnpm target:prune`.
- `tests/`: the Vitest tests. `tests/test_repository.ts` and `tests/sample_cargo_project.ts` write temporary repositories and a small Rust package, and only the tests import them.
- Command to test this folder: `pnpm --filter worktree_tools test`

## Rules
- `src/worktree_git.ts` is the only code that runs `git`, and `src/target_paths.ts` is the only place that knows where the Cargo target folder is inside a checkout.
- A worktree is always created inside the folder `.agents/worktrees` of the main checkout, on the same disk, which the copy-on-write clone needs. When the clone is not possible, the worktree is still created and the command says why.
- After the clone, `pnpm worktree:create` gives the unchanged sources the dates of the main checkout (`SourceDates`), and writes the path of the worktree in the build script records (`BuildScriptOutputs`). It never changes the date of a file whose content differs, so Cargo still compiles every real change. The option `--without-optimization` skips all of these steps.
- `pnpm worktree:create` ends with `pnpm install --frozen-lockfile`. A failure keeps the worktree, prints the error, and exits with the code 1.
- The tests use a fake `pnpm` first in `PATH` (`TestRepository.pathWithFakePnpm`), except the tests that check the installation.
- The cleaning tool and the pruning tool delete nothing without the option `--yes`. The cleaning tool never deletes the target folder of the main checkout, the first checkout of `git worktree list`.
- The pruning tool runs each Cargo command of `TargetPrune.cargoCommands` in the folder of its package, like the scripts of its `package.json`, because Cargo reads the `.cargo/config.toml` of a package only there. It keeps a file only by the hash of a unit that the Cargo log names, and deletes nothing when a Cargo command fails or when the log names no unit.
- No Rust package of this repository imports this package.

## Background
- The package is a copy of the package `worktree_tools` of Heidrun Code ([issue 209](https://github.com/heidrun-org/heidrun_code/issues/209), [issue 864](https://github.com/heidrun-org/heidrun_code/issues/864), [issue 877](https://github.com/heidrun-org/heidrun_code/issues/877)), without the parts that only Heidrun Code needs. It came to this repository in [issue 14](https://github.com/heidrun-org/localai_rust/issues/14).
