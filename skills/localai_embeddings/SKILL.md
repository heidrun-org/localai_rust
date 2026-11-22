---
name: localai_embeddings
description: Compute local text embeddings for text or UTF-8 files, save embedding vectors, or compare two texts with Heidrun Local AI Rust.
---

# localai_embeddings

Use this tool to turn text into numeric vectors for similarity search, grouping, or comparing two passages. It accepts text and UTF-8 files, splits long documents into chunks, and returns a vector for each document and chunk. Use the local executable to answer the request.

## When to use this tool

Use embeddings when the task needs a numeric representation of text. For one similarity comparison, use `--compare` and report the score. For a search across many documents, compute and save the document vectors once, then embed each new query with the same model and compare it against those vectors. Use the same model and compatible chunking settings across the collection so the results are comparable.

Use the full document vector to rank documents. Use the chunk vectors and offsets to locate the matching passage in a long document. To embed many files, pass repeated `--file` options and keep the returned `source` and `input_index` with each result.

## Run from the user's project

Use the executable on PATH or the absolute path printed by the setup commands. A first inference may download missing model files after approval. Reuse the installed executable and model on later requests. Return the executable's actual output, or explain the blocker if no output is available.

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
- `--benchmark N` measures N rounds after one warm-up round and excludes model loading. `-v` or `--verbose` writes progress and timing information to standard error. `-h` or `--help` lists the options provided by the installed executable.

### Find the executable and model

Look for `localai_embeddings` on PATH and at `<setup directory>/bin/localai_embeddings`. The setup directory is `LOCALAI_SETUP_DIR` when set, otherwise `${XDG_DATA_HOME:-$HOME/.local/share}/heidrun-localai-rust`. If the executable and model are ready, run the executable directly.

The default model is `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` (about 460 MiB). Resolve the cache as `HF_HUB_CACHE`, then `HUGGINGFACE_HUB_CACHE`, then the `hub` directory under `HF_HOME` or under the Hugging Face cache in `XDG_CACHE_HOME` (default `~/.cache`). In the repository's `models--owner--name` cache directory, read `refs/main` and check that the following files resolve under `snapshots/<revision>/`: config.json, tokenizer.json, model.safetensors, 1_Pooling/config.json, and sentence_bert_config.json. The last two files may also have a matching negative-cache marker under `.no_exist/<revision>/`. For a different model check its exact repository and files. Treat an uncertain or incomplete cache as a possible download.

Use filesystem reads for these checks. Even a partial cache can trigger a download. Leave model update checks off unless the user requests and approves them.

## Ask before setup

If a build is needed, check for Git, a native compiler and linker. Look for mise on PATH and at `<setup directory>/bin/mise`. If mise is missing, check for curl on macOS or Linux. If the managed Rust toolchain is missing, include the Rust download. For a missing prerequisite, identify the installation command or explain what the human needs to do if the environment prevents setup.

Ask the human for explicit approval before installing a prerequisite, fetching source, building, or downloading a model. List only the needed steps, the setup directory, and the model name and size when known. Explain mise when it needs installation:

> Mise can download and run the tools a command needs without adding them to your usual shell setup. It keeps the downloads for reuse.

Ask “May I proceed with these steps?” and wait. On refusal, leave setup untouched. Ask again if an additional setup step turns out to be necessary.

## Install the executable with mise

When the executable is missing, run the following in a fresh POSIX shell after approval. The commands reuse mise if available or install the official release on macOS and Linux. Rust and Cargo use a dedicated setup directory. Mise fetches and builds only this package from GitHub.

```sh
# Stop on errors or unset variables so setup cannot continue after a failed step.
set -eu
package=localai_embeddings
# Keep downloads and builds in a dedicated directory. An absolute path still
# points to the same place after the script changes its working directory.
setup_dir="${LOCALAI_SETUP_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/heidrun-localai-rust}"
case "$setup_dir" in
    /*) ;;
    *) echo 'LOCALAI_SETUP_DIR must be an absolute path.' >&2; exit 2 ;;
esac

# If the managed executable already exists, report its path and skip setup.
if [ -x "$setup_dir/bin/$package" ]; then
    printf '%s\n' "$setup_dir/bin/$package"
    exit
fi

# Reuse mise from PATH or from this setup directory before downloading it.
mkdir -p "$setup_dir/$package" "$setup_dir/bin"
if command -v mise >/dev/null 2>&1; then
    mise_bin="$(command -v mise)"
else
    mise_bin="$setup_dir/bin/mise"
    if [ ! -x "$mise_bin" ]; then
        case "$(uname -s)" in
            Darwin|Linux) ;;
            *) echo 'Install mise for this platform and put it on PATH. See https://mise.jdx.dev/installing-mise.html' >&2; exit 1 ;;
        esac
        # Download the official installer to a temporary file and remove that
        # file when the shell exits. Install mise without shell activation.
        installer="$(mktemp "$setup_dir/mise-installer.XXXXXX")"
        trap 'rm -f "$installer"' EXIT
        curl -fsSL https://mise.run -o "$installer"
        MISE_INSTALL_PATH="$mise_bin" MISE_VERSION=v2026.11.12 MISE_INSTALL_HELP=0 sh "$installer"
    fi
fi

# Run mise from the package setup directory. Keep mise data, Cargo files,
# and Rust toolchains separate from the user's normal configuration.
cd "$setup_dir/$package"
export MISE_DATA_DIR="$setup_dir/mise-data"
export MISE_CACHE_DIR="$setup_dir/mise-cache"
export MISE_CONFIG_DIR="$setup_dir/mise-config"
export MISE_STATE_DIR="$setup_dir/mise-state"
export CARGO_HOME="$setup_dir/cargo"
export CARGO_TARGET_DIR="$setup_dir/target"
export RUSTUP_HOME="$setup_dir/rustup"
export MISE_RUST_CARGO_HOME="$CARGO_HOME"
# Trust the setup directory for this run so mise can read the file below.
export MISE_TRUSTED_CONFIG_PATHS="$PWD"
# Pin the Rust version and the Git revision. The crate name selects just
# the requested executable from the repository.
cat > mise.toml <<EOF
[tool_alias]
$package = "cargo:https://github.com/heidrun-org/localai_rust"

[tools]
rust = "1.100.0"
$package = { version = "rev:a77075be2e6d981ea5eb6588d2695728b4da59dd", crate = "$package" }
EOF
# Let mise fetch the source and build the executable with Cargo.
"$mise_bin" install --jobs 1
# Give later requests a stable executable path that works from any project.
executable="$("$mise_bin" where "$package")/bin/$package"
test -x "$executable"
ln -sfn "$executable" "$setup_dir/bin/$package"
printf '%s\n' "$setup_dir/bin/$package"
```
