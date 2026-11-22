---
name: localai-rag
description: Search named collections of local Markdown and plain-text documents by meaning, find supporting passages, and cite exact local sources. Use when a user asks to find evidence in local documents, list saved collections, or create or refresh a Local AI RAG collection.
---

# Local AI RAG

Use the local `localai_rag` executable. The tool embeds text and searches on the user's computer. Use a saved collection for search. Create or refresh a collection when the user requests indexing or an update.

## Search and cite

Use the collection name and data directory supplied by the user. Otherwise run `localai_rag list` to show names, source directories, models, and last successful refresh times. If several collections fit and the choice matters, ask which one to use. If a collection has never been refreshed, explain that it needs to be indexed before it can yield results.

Use the same `--data-directory PATH` or `LOCALAI_RAG_HOME` override for every operation on a collection. The default is `heidrun-localai-rust/rag-collections` under the user's local application data directory. The commands return JSON.

```sh
localai_rag list
localai_rag search --collection "$collection" --query "$query" --limit 5 --chunks-per-document 2
```

Read the returned chunk text. Cite the document's `source_path` with the chunk's `start_line` and `end_line`, and quote the exact returned text when asked. The line numbers are one-based and inclusive. If the file may have changed since indexing, compare the current file hash to `content_hash` before citing the live line numbers. Describe a changed or missing source as an indexed version, or refresh if the user wants current results.

Similarity scores rank matches within the collection. There is no minimum score. Use the excerpt itself to judge whether it supports the answer, mark tentative matches, and say when the collection contains no relevant evidence. A search answer can cite multiple excerpts when each supports a distinct claim.

## Create or refresh

```sh
localai_rag create --collection "$collection" --root "$source_directory"
localai_rag sync --collection "$collection"
```

Create records the directory and pins the exact model revision. Sync recursively indexes UTF-8 Markdown and plain-text files, reuses unchanged embeddings, and removes stale chunks. A failed refresh leaves the previous index available. If an older collection reports that the revision is unpinned, explain that `sync` will rebuild the collection and pin its model revision.

## Check availability before setup

Look for `localai_rag` on PATH, an executable in the user's checkout, or the absolute path recorded at `<cgx data>/localai/localai_rag.path`. For a saved cgx path, check that it names an executable `localai_rag` under `<cgx data>/bins/`. Reuse the executable and model when available. Listing collections needs no model. Creating a collection may download the selected model. Searching or refreshing a saved collection may download files from its pinned model revision if the files are missing.

The default model for a new collection is `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` (about 450 MiB for the weights). Use `--model OWNER/NAME` on creation when the user requests a model supported by Local AI Embeddings. Check the Hugging Face cache with read-only commands. The cache uses `HF_HUB_CACHE`, then `HUGGINGFACE_HUB_CACHE`, then the `hub` folder under `HF_HOME` or the default Hugging Face cache. For a saved collection, if necessary read the `revision` in the `store_metadata` table of its `vectors/*/store.sqlite3` using a read-only SQLite connection. Check that revision's snapshot for `config.json`, `tokenizer.json`, `model.safetensors`, and the optional `1_Pooling/config.json` and `sentence_bert_config.json` files or their cached absence. Treat uncertain cache contents as a possible download.

If the executable is missing, check for cgx on PATH or at `<cgx data>/runner/cgx`, Git, a native compiler and linker, and a compatible Rust compiler and Cargo. If cgx is missing, check for curl, tar, and a checksum tool. On macOS and Linux, cgx uses `${XDG_CACHE_HOME:-$HOME/.cache}/cgx` for source and `${XDG_DATA_HOME:-$HOME/.local/share}/cgx` for builds and executables. A cgx command that prints an executable path can fetch and build the package.

Ask one question before any required prerequisite installation, source fetch, Cargo download, build, or model download. State the missing steps, where they write, the model and size if known, and that approval covers those steps and the requested operation. If the user declines, leave setup untouched. Reuse approved setup on later requests.

## Build with cgx after approval

Reuse cgx when available. Otherwise download the [cgx 0.2.0 archive](https://github.com/anelson/cgx/releases/tag/v0.2.0) and checksum for the user's platform, verify it, and extract the runner to `<cgx data>/runner/cgx`. Reuse the current Rust toolchain when compatible. If Rust is missing and installation was approved, install the minimal Rust 1.100.0 toolchain with [Rustup](https://rust-lang.github.io/rustup/installation/index.html). Use `--no-modify-path` and set `RUSTUP_HOME` and `CARGO_HOME` to the `rustup` and `cargo` subdirectories of cgx's data directory. Use that Rustup home and put its Cargo `bin` directory on PATH for the build subprocess.

Set `cgx` to the runner's executable path. When using a separately installed toolchain, pass `+1.100.0` immediately after the runner path.

```sh
cgx_data="${XDG_DATA_HOME:-$HOME/.local/share}/cgx"
tool_path=$(CARGO_HOME="$cgx_data/cargo" "$cgx" \
    --git https://github.com/heidrun-org/localai_rust.git --branch main \
    --prebuilt-binary never --no-exec localai_rag) || exit 1
test -x "$tool_path" || exit 1
case "$tool_path" in
    "$cgx_data"/bins/*/localai_rag)
        mkdir -p "$cgx_data/localai"
        printf '%s\n' "$tool_path" > "$cgx_data/localai/localai_rag.path"
        ;;
esac
```

Run the returned executable by its absolute path from the user's project. For a supplied review checkout, replace the Git source and branch options with `--path "$checkout/packages/localai_rag"`.
