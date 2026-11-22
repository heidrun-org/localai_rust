---
name: localai_decisions
description: Run a local yes-or-no or multiple-choice decision and report the selected option and probabilities with Heidrun Local AI Rust.
---

# localai_decisions

Use this tool to answer questions about supplied information with a local decision model. It can choose among listed answers, estimate the probability of a yes answer, or score an item against ordered levels. Use the local executable to answer the request.

## When to use this tool

Use this tool when the possible outcomes are already known. A choice question works for routing a request to a team. A yes-or-no question works for checking whether a stated condition applies. A score question works when the user can describe an ordered scale, such as levels of urgency. Write the question and choices so the decision can be made from the supplied information.

Use the short form for one text and one question. Use a JSON request when the input is already structured or several questions should use the same information.

## Run from the user's project

Use the executable on PATH or the absolute path printed by the setup commands. A first inference may download missing model files after approval. Reuse the installed executable and model on later requests. Return the executable's actual output, or explain the blocker if no output is available.

### Inputs and output

For one question, use `--text TEXT` for the information to evaluate and `--question QUESTION` for the question. Add `--choices A,B,C` for multiple choice. Leave out `--choices` to ask a yes-or-no question. Use `--output PATH` to save the JSON response.

```sh
localai_decisions --text 'I was charged twice.' --question 'Which team should help?' --choices billing,shipping,technical
localai_decisions --text 'I was charged twice.' --question 'Is this about a payment?'
localai_decisions --request request.json --output decision.json
```

For structured inputs or multiple questions, pass `--request PATH` or pipe the JSON to standard input with neither `--request` nor `--text`. The `state` can be a text string, an object, or an array. Give each question a key and its `type` and `instructions`. For `choice`, set `criteria` to a map of options with descriptions or `null`. For `score`, set `criteria` to 2–10 ordered labels. For a yes-or-no question, use `noul` and omit `criteria` or describe the true and false options.

```json
{
  "state": "I was charged twice.",
  "questions": {
    "team": {
      "type": "choice",
      "instructions": "Which team should help?",
      "criteria": {"billing": null, "shipping": null, "technical": null}
    },
    "payment": {"type": "noul", "instructions": "Is this about a payment?"}
  }
}
```

Read the results under `answers` using the original question keys. Choice results include the selected `choice` and `probabilities`. A `noul` result gives the probability of yes. Score results include the expected `score`, the ordered labels in `legend`, and `probabilities`.

### Interpret the result

Report the selected answer and its probability, along with any other probabilities the user requests. For a yes-or-no question, label the `noul` number as the probability of yes. For a score, use the ordered labels from `legend` when explaining the numeric result. Keep model probabilities distinct from any decision rule the user supplied.

### Command options

- `--model OWNER/REPO[:QUANTIZATION]` selects a compatible remote decision model. A local `.gguf` model path also works. `--check-model-updates` can download new model files and requires approval.
- `--device cpu|gpu` (also `--backend`) chooses the processor or graphics hardware. Use `cpu` with the installation described below. Graphics hardware requires a compatible build. `--threads N` sets the processor thread count. `--batch-size N` sets the maximum tokens per evaluation and defaults to 2048.
- `--verbose` writes model logs and timings to standard error. `-h` or `--help` lists the installed options. `-V` or `--version` prints the version.
- The `benchmark` command times built-in short and long cases after warm-up. Use `--decisions N` to set the number of decisions per case and `--json` for machine-readable results. Run `localai_decisions benchmark --help` for the benchmark options.

### Find the executable and model

Look for `localai_decisions` on PATH and at `<setup directory>/bin/localai_decisions`. The setup directory is `LOCALAI_SETUP_DIR` when set, otherwise `${XDG_DATA_HOME:-$HOME/.local/share}/heidrun-localai-rust`. If the executable and model are ready, run the executable directly.

The default model is `ggml-org/Kev-0.8B-GGUF` (about 0.81 GB). Resolve the cache as `HF_HUB_CACHE`, then `HUGGINGFACE_HUB_CACHE`, then the `hub` directory under `HF_HOME` or under the Hugging Face cache in `XDG_CACHE_HOME` (default `~/.cache`). In the repository's `models--owner--name` cache directory, read `refs/main` and check that the following files resolve under `snapshots/<revision>/`: Kev-0.8B-Q8_0.gguf. A user-supplied local `.gguf` file can be used with `--model` after checking it exists. For a different model check its exact repository and files. Treat an uncertain or incomplete cache as a possible download.

Use filesystem reads for these checks. Even a partial cache can trigger a download. Leave model update checks off unless the user requests and approves them.

## Ask before setup

If a build is needed, check for Git, a native compiler and linker, and CMake. Look for mise on PATH and at `<setup directory>/bin/mise`. If mise is missing, check for curl on macOS or Linux. If the managed Rust toolchain is missing, include the Rust download. For a missing prerequisite, identify the installation command or explain what the human needs to do if the environment prevents setup.

Ask the human for explicit approval before installing a prerequisite, fetching source, building, or downloading a model. List only the needed steps, the setup directory, and the model name and size when known. Explain mise when it needs installation:

> Mise can download and run the tools a command needs without adding them to your usual shell setup. It keeps the downloads for reuse.

Ask “May I proceed with these steps?” and wait. On refusal, leave setup untouched. Ask again if an additional setup step turns out to be necessary.

## Install the executable with mise

When the executable is missing, run the following in a fresh POSIX shell after approval. The commands reuse mise if available or install the official release on macOS and Linux. Rust and Cargo use a dedicated setup directory. Mise fetches and builds only this package from GitHub.

```sh
# Stop on errors or unset variables so setup cannot continue after a failed step.
set -eu
package=localai_decisions
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
