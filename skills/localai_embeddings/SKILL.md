---
name: localai_embeddings
description: Compute local text embeddings for text or UTF-8 files, save embedding vectors, or compare two texts with Heidrun Local AI Rust.
---

# localai_embeddings

Use this tool to turn text into numeric vectors for similarity search, grouping, or comparing two passages. It accepts text and UTF-8 files, splits long documents into chunks, and returns a vector for each document and chunk. Use the local executable to answer the request.

## When to use this tool

Use embeddings when the task needs a numeric representation of text. For one similarity comparison, use `--compare` and report the score. For a search across many documents, use the `localai_vector_store` skill to save the document vectors once, reopen the store, and search with new query embeddings from the same model. Use the same model and compatible chunking settings across the collection so the results are comparable.

Save the embedding output with `--output FILE` for use with Local AI Vector Store. The output provides the model identifier, number of values, and source names. Declare the model provider and supply the revision when known.

Use the full document vector to rank documents. Use the chunk vectors and offsets to locate the matching passage in a long document. To embed many files, pass repeated `--file` options and keep the returned `source` and `input_index` with each result.

## Run from the user's project

Use the executable on PATH or the absolute path returned by cgx. A first inference may download missing model files after approval. Reuse the installed executable and model on later requests. Return the executable's actual output, or explain the blocker if no output is available.

### Inputs and output

Pass `-t` or `--text TEXT` for literal text, or `-f` or `--file PATH` for a UTF-8 file. Repeat either option for multiple inputs. With neither option, pipe text into the command or paste it and finish with Control-D. Extract text from binary files before passing it to the tool.

The command writes JSON with `model`, `backend`, `dimensions`, and `documents`. Read `documents[].global_vector` for a document’s vector or `documents[].chunks` for the text, offsets, token count, and vector of each chunk. The default model produces 384 values per vector. Use `--output PATH` to save the JSON to a file. With `--compare`, supply exactly two inputs and read `cosine_similarity` from the result.

```sh
localai_embeddings --text 'The library opens at nine.' --output embedding.json
localai_embeddings --file notes.md --chunking markdown --output notes-embeddings.json
localai_embeddings --compare --text 'The cat sleeps on the chair.' --text 'A cat rests on a seat.'
```

### Interpret the result

A larger cosine similarity means the model considers the passages more similar. Report the score or the ranking, and keep the raw vector in the requested file when the user wants to use it in another tool. When comparing short text with a long document, chunk matches may be more informative than the overall document score.

### Command options

- `--chunking auto|text|markdown|lines|code` chooses how to split documents. The default, `auto`, infers the format from the file extension. `--overlap-tokens N` sets the maximum repeated tokens between adjacent chunks and defaults to zero.
- `--model OWNER/REPO` selects a compatible BERT sentence embedding model with mean pooling. `--check-model-updates` checks for a newer copy and can download files, so use it only after approval.
- `--backend cpu|cuda|metal|wasm` chooses the runtime. Use `cpu` with the installation described below. CUDA and Metal require compatible hardware and build features. WebAssembly needs a separate browser build. `--threads N` sets the number of processor workers.
- `--benchmark --duration 10s` measures mebibytes of original input text per second after one warm-up pass. The duration defaults to 10 seconds. `-v` or `--verbose` writes progress and timing information to standard error. `-h` or `--help` lists the options provided by the installed executable.

## Choose a model by download size

These sentence embedding models use BERT and average the token vectors, which is the pooling method supported by this tool. Sizes are for the model weights. Tokenizer and settings files add a small download.

| Model for `--model` | Download | When to choose it |
| --- | ---: | --- |
| [`sentence-transformers/all-MiniLM-L6-v2`](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/main/model.safetensors) | 91 MB | Small English model for a first similarity search. Produces 384 values per vector. |
| [`intfloat/e5-small-v2`](https://huggingface.co/intfloat/e5-small-v2) | 133 MB | Compact English model for matching search queries to passages. Produces 384 values. |
| [`intfloat/e5-base-v2`](https://huggingface.co/intfloat/e5-base-v2) | 438 MB | English search model with 768 values per vector. Evaluate it if you have room for a larger model and index. |
| [`sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`](https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/blob/main/model.safetensors) | 471 MB | Default for English, French, and other supported languages. Produces 384 values. |
| [`intfloat/e5-large-v2`](https://huggingface.co/intfloat/e5-large-v2) | 1.34 GB | Larger English search model with 1,024 values per vector. Compare it with the smaller E5 models on your own queries. |

For an E5 search, prefix the search text with `query: ` and the passages with `passage: `. If you switch models, recompute vectors already stored in the same index.

## Check what is already available

Use read-only filesystem checks before setup. Look for `localai_embeddings` on PATH or read the saved executable path at `<cgx data>/localai/localai_embeddings.path`. Verify that the saved path points to an executable named `localai_embeddings` under `<cgx data>/bins/`. If the saved path is absent, reuse a cached executable when you can verify it came from the selected Heidrun GitHub repository. If the executable and all model files are ready, run the executable directly from the user's project.

On macOS and Linux, the cgx cache is `${XDG_CACHE_HOME:-$HOME/.cache}/cgx` and `<cgx data>` is `${XDG_DATA_HOME:-$HOME/.local/share}/cgx`. cgx keeps downloaded source in its cache and build files and executables under `build/` and `bins/` in its data directory.

Use `HF_HUB_CACHE` for the Hugging Face hub cache when set, then `HUGGINGFACE_HUB_CACHE`. Otherwise use the `hub/` directory under `HF_HOME` or under `${XDG_CACHE_HOME:-$HOME/.cache}/huggingface`. Each repository has a folder named `models--owner--name`.

The default model is `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`. In the cache directory for the selected model, read `refs/main` and check the files at `snapshots/<revision>/`. The required files are `config.json`, `tokenizer.json`, and `model.safetensors`. Check `1_Pooling/config.json` and `sentence_bert_config.json` as well. The latter two are optional, so a corresponding absence marker in `.no_exist/<revision>/` also counts as cached. A missing file with no absence marker can cause a download.

Treat incomplete or uncertain model caches as a possible download. Include any requested model update in the approval question because the update can download files. Do not use a cgx command for preflight if it could fetch or build the tool, even when the command would only print the executable path.

## Ask once before setup

If the tool needs a build, check for `cgx` on PATH or at `<cgx data>/runner/cgx`. Check for Git, a compiler, a linker, and a compatible Rust compiler and Cargo. With Rustup, use `rustup toolchain list` and `rustup run TOOLCHAIN rustc --version` to check an existing toolchain without updating it. If cgx is missing, check for curl, tar, and a checksum tool. Identify any other missing prerequisite and how it would be obtained before asking.

Ask **one** question and wait for explicit approval. List every required setup action, including downloads of cgx or prerequisites, the GitHub source fetch, Cargo dependency downloads, the build, and any model download. Give the relevant directories and the model name and download size when known. Explain both choices in the same question.

- **Approve** — Complete the listed setup and run the requested operation. One approval covers all listed steps.
- **Decline** — Leave setup untouched and explain which missing item prevents the requested operation.

If an unexpected requirement falls outside the approved steps, report the blocker and stop. Keep the user's existing default Rust toolchain intact.

## Set up and run with cgx

After approval, reuse cgx if available. Otherwise, select the published [cgx 0.2.0 archive](https://github.com/anelson/cgx/releases/tag/v0.2.0) for the operating system and processor. For example, Apple Silicon macOS uses `cgx-aarch64-apple-darwin.tar.xz`. Download the archive and its checksum into a temporary directory, verify the archive against the published checksum, and extract `cgx` to `<cgx data>/runner/cgx`. Invoke the runner by its absolute path. This keeps the bootstrap in the default cgx data directory. Remove the temporary archive after extraction.

Use an existing compatible Rust toolchain when available. If Rust is absent or incompatible and the user approved a separate installation, use the official [Rustup standalone installer](https://rust-lang.github.io/rustup/installation/index.html) with `--no-modify-path --profile minimal --default-toolchain 1.100.0`, `RUSTUP_HOME` under `<cgx data>/rustup`, and `CARGO_HOME` under `<cgx data>/cargo`. For the build subprocess, set the isolated `RUSTUP_HOME` and prepend the isolated Cargo `bin/` directory to its PATH. For a prerequisite without an approved installation method, report the blocker.

On macOS and Linux the commands below use cgx's default directories. Set `cgx` to the available executable path and `toolchain` to the installed Rustup toolchain name. For an existing Cargo installation without Rustup, omit the `+` argument and point `CARGO` at that Cargo executable. Cargo downloads go under `<cgx data>/cargo` for the skill's build. Use the same isolated `RUSTUP_HOME` on later builds if a separate Rust installation was needed.

```sh
cgx_data="${XDG_DATA_HOME:-$HOME/.local/share}/cgx"
tool_path=$(CARGO_HOME="$cgx_data/cargo" "$cgx" "+$toolchain" \
    --git https://github.com/heidrun-org/localai_rust.git --branch main \
    --prebuilt-binary never --no-exec localai_embeddings) || exit 1
test -x "$tool_path" || exit 1
mkdir -p "$cgx_data/localai"
printf '%s\n' "$tool_path" > "$cgx_data/localai/localai_embeddings.path"
printf '%s\n' "$tool_path"
```

Run the printed executable from the user's project with the arguments for the request. Model downloads use the Hugging Face cache described above. For a later request, use the saved executable and cached model after checking both. Return actual output or the specific blocker.

For cleanup, list cached executables with `find "$cgx_data/bins" -type f -perm -100`. Remove the cgx cache and data directories when you want to discard their source, builds, tools, and Cargo downloads. The Hugging Face cache stores the model files separately.
