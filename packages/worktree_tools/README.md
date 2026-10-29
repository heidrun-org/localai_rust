# worktree_tools

Three command line tools that make a new Git worktree of this repository fast and small on the disk. Without them, each new worktree has an empty Cargo target folder, and its first `pnpm test` compiles every Rust dependency again (llama.cpp, Candle, tokenizers): it takes about 198 seconds and about 3 GB of disk space, and it writes these gigabytes to the disk each time.

The tools come from the package `worktree_tools` of [Heidrun Code](https://github.com/heidrun-org/heidrun_code), which solved the same problem ([issue 864](https://github.com/heidrun-org/heidrun_code/issues/864)).

## What it contains

- `src/worktree_create_cli.ts`: the command line `pnpm worktree:create`.
- `src/target_clean_cli.ts`: the command line `pnpm target:clean`.
- `src/target_prune_cli.ts`: the command line `pnpm target:prune`.
- `src/index.ts`: the public interface for another package: the classes `WorktreeCreate`, `TargetClean`, `TargetPrune`, `DirectoryClone`, `SourceDates`, and `BuildScriptOutputs`.
- `src/worktree_git.ts`: the only code that runs `git`.
- `src/target_paths.ts`: the place of the Cargo target folder inside a checkout.
- `tests/`: the Vitest tests, and helpers that create temporary Git repositories and a small Rust package.

## Create a worktree

Run the command from the root of the repository, or from any worktree.

```sh
pnpm worktree:create fix-thing-12                 # worktree on a new branch, from the current commit
pnpm worktree:create fix-thing-12 --base main     # worktree on a new branch, from the branch main
pnpm worktree:create fix-thing-12 --without-optimization
```

The command creates the worktree `.agents/worktrees/<name>` of the main checkout, on a new branch with the same name. Then, on macOS:

1. It clones the Cargo target folder `target` of the main checkout into the worktree, with one call to the function `clonefile` of macOS, through `python3` (class `DirectoryClone`). The clone is copy-on-write: it shares the disk blocks with the main checkout until Cargo rewrites a file. When `python3` is missing or `clonefile` fails, the command runs `cp -c -R -p`.
2. It gives each tracked file with the same content as in the main checkout the date of the main checkout, and also each folder whose list of names is the same (class `SourceDates`). Git gives every file of a new worktree the date of the creation, and Cargo then compiles again every crate of the repository.
3. It replaces the absolute path of the main checkout with the path of the worktree in the build script records `target/<debug or release>/build/*/output`, for the folder `packages` and for the folder `target`, and keeps the date of each record (class `BuildScriptOutputs`). The build script of `llama-cpp-sys-2` writes there the absolute path of the compiled llama.cpp library.

At the end, the command runs `pnpm install --frozen-lockfile` in the worktree, so `pnpm test` runs at once. When the installation fails, the command prints the error of `pnpm`, keeps the worktree, and exits with the code 1.

The option `--without-optimization` only creates the worktree and installs the packages. The first `pnpm test` then compiles everything. Use it to check whether a problem comes from these optimizations.

Measured on 2026-10-29, on a copy of the repository built once:

| Step | Before | After |
| --- | --- | --- |
| Creation of the worktree | - | 2.3 seconds |
| First `pnpm test` in the new worktree | 198 seconds, every crate compiled | 25 seconds, 0 crates compiled |
| Disk space of the new worktree before the first change | about 3 GB | 5 MB |
| `pnpm test` after a change of one line in `localai_llm` | - | 20 seconds, only `localai_llm` compiled |

Limits:

- The main checkout must be built first, with `pnpm test` or the script `build` of a package. Without the folder `target/debug` or `target/release`, the command creates the worktree without the clone and says so.
- The clone needs macOS and the Apple File System, with both folders on the same disk. A worktree inside the main checkout is always on the same disk.
- Run Cargo in the folder of a package, as the scripts of each `package.json` do, and never at the root of the repository. At the root, Cargo builds the three packages together, which gives the shared dependencies other features, and it does not read the file `.cargo/config.toml` of `localai_decisions` and `localai_llm`, which sets `GGML_METAL=OFF`. Both changes compile everything again.
- The installation needs the command `pnpm` in the `PATH`.

## Delete the target folders of old worktrees

```sh
pnpm target:clean               # list the target folders that no build used for 7 days
pnpm target:clean --days 3      # the same, for 3 days
pnpm target:clean --yes         # delete them
```

- The age of a target folder is the most recent modification time of any file or folder inside it.
- The command never touches the main checkout, the first checkout of `git worktree list`.
- The command does not show the size of the folders. The size that `du` shows is too high, because the clones share their blocks.

## Delete the obsolete files of the target folder

```sh
pnpm target:prune          # list the files that the current builds do not use
pnpm target:prune --yes    # delete them
```

Cargo never deletes the files of an old build, so the folders `target/debug` and `target/release` grow without end, and each clone takes longer. The command runs, in the folder of each package, the Cargo commands of its scripts `build`, `cli`, and `test` (`TargetPrune.cargoCommands`), with `CARGO_LOG=cargo::core::compiler::fingerprint=debug`. The log names the folder `.fingerprint/<crate>-<hash>` of every compilation unit that a build uses. Each file of `deps`, `.fingerprint`, and `build` has the hash of its unit at the end of its name, so the command lists every entry with another hash. In `incremental`, the names hold no hash, so the command keeps the 2 most recently changed folders of each crate. It never touches another folder.

Limits:

- The Cargo commands compile what is out of date, so the command can take minutes. When a Cargo command fails, or when the log names no unit, the command deletes nothing and exits with the code 1.
- The command works on the checkout that holds the current folder, the main checkout or a worktree.
- A build of a kind that the command does not run, for example `cargo check`, loses its files. To keep them, add the Cargo command to `TargetPrune.cargoCommands`.

## Test

```sh
pnpm --filter worktree_tools test
```
