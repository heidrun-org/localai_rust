---
name: localai_vector_store
description: Create, list, inspect, populate, or search a persistent Local AI Vector Store. Use for similarity search over a saved collection or when the user already has embeddings to store or query.
---

# localai_vector_store

Use the local executable to create or search a store. Use the `localai_embeddings` skill when an input is still text or a document that needs an embedding. If the user already has compatible embedding files, use those files directly.

## Select the store

Use the caller's store identifier and parent directory when supplied. Otherwise, `localai_vector_store list` shows the stores in the default application data directory. Use `--parent-dir PATH` when the caller keeps stores elsewhere. If several stores match the request and choosing among them matters, ask which store to use. `describe --store ID` shows the store's description, model details, and embedding count.

## Create, add, and search

The commands print JSON. Replace the example values with the actual paths, model information, and store identifier.

```sh
localai_vector_store create --description 'Project notes' --model-provider huggingface \
  --model sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 --dimensions 384
localai_vector_store add --store "$store_id" --input documents.json --model-provider huggingface
localai_vector_store search --store "$store_id" --query query.json --model-provider huggingface --limit 5
```

- On creation, read the model identifier and number of values from the embedding file's `model` and `dimensions` fields and use the generated `identifier` for later operations. Keep the same parent directory across commands.
- Declare the actual model provider. Use `--model-revision` on `create`, `add`, and `search` when the revision is known. A store with an unknown revision accepts an input with an unknown revision if the provider, model identifier, and vector length match.
- By default, `add` saves the whole-document vector from each document. Use `--vectors chunk` or `--vectors both` when requested. Use `--source-prefix` to distinguish unrelated inputs with the same source name. A duplicate embedding identifier rejects the entire import. Check the source names and store before retrying.
- For `search`, pass a file containing one query document embedded with the store's model. Return the actual source references and cosine similarity scores in the order returned by the tool. Reuse the saved document embeddings for later searches.

## Check what is already available

Look for `localai_vector_store` on PATH or read the saved executable path at `<cgx data>/localai/localai_vector_store.path`. Verify that the saved path points to an executable named `localai_vector_store` under `<cgx data>/bins/`. Reuse the executable directly from the user's project when available. Local AI Vector Store itself needs no model download.

On macOS and Linux, cgx keeps source under `${XDG_CACHE_HOME:-$HOME/.cache}/cgx` and uses `${XDG_DATA_HOME:-$HOME/.local/share}/cgx` for its data, including builds and executables. Use read-only checks before setup. A cgx command that prints the executable path can also fetch and build the package.

## Ask once before setup

Check for cgx on PATH or at `<cgx data>/runner/cgx`, Git, a native compiler and linker, and a compatible Rust compiler and Cargo. With Rustup, use `rustup toolchain list` and `rustup run TOOLCHAIN rustc --version` to check without updating. If cgx is missing, check for curl, tar, and a checksum tool.

Ask one question for explicit approval before fetching source, installing prerequisites, downloading Cargo dependencies, or building. List only the required steps and their storage locations. Approval covers those steps and the requested operation. If the user declines, leave setup untouched. Keep the existing default Rust toolchain. For a separate embedding request, include any needed model download in the same approval question.

## Set up and run with cgx

After approval, reuse cgx or select the published [cgx 0.2.0 archive](https://github.com/anelson/cgx/releases/tag/v0.2.0) for the operating system and processor. For example, Apple Silicon macOS uses `cgx-aarch64-apple-darwin.tar.xz`. Download the archive and checksum to a temporary directory, verify it against the published checksum, and extract `cgx` to `<cgx data>/runner/cgx`. Invoke it by its absolute path.

Reuse a compatible Rust toolchain. If none is available and installation was approved, use the official [Rustup standalone installer](https://rust-lang.github.io/rustup/installation/index.html) with `--no-modify-path --profile minimal --default-toolchain 1.100.0`. Set `RUSTUP_HOME` to `<cgx data>/rustup` and `CARGO_HOME` to `<cgx data>/cargo` for the installation. Use the same isolated Rustup directory and put the isolated Cargo `bin/` directory on PATH for the build subprocess.

Set `cgx` to the executable path and `toolchain` to the installed Rustup toolchain name. With a compatible Cargo installation outside Rustup, omit the `+$toolchain` argument and set `CARGO` to the Cargo executable.

```sh
cgx_data="${XDG_DATA_HOME:-$HOME/.local/share}/cgx"
tool_path=$(CARGO_HOME="$cgx_data/cargo" "$cgx" "+$toolchain" \
    --git https://github.com/heidrun-org/localai_rust.git --branch main \
    --prebuilt-binary never --no-exec localai_vector_store) || exit 1
test -x "$tool_path" || exit 1
mkdir -p "$cgx_data/localai"
printf '%s\n' "$tool_path" > "$cgx_data/localai/localai_vector_store.path"
```

Run `"$tool_path"` from the user's project and reuse the saved executable path on later requests.

For an existing review checkout, use `--path "$checkout/packages/localai_vector_store"` in place of the Git source and branch arguments. Run the returned checkout executable directly. Save a path to the cgx data directory only when the executable is under its `bins/` directory.
