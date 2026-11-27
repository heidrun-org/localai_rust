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

Use the executable on PATH or the absolute path returned by cgx. A first inference may download missing model files after approval. Reuse the installed executable and model on later requests. Return the executable's actual output, or explain the blocker if no output is available.

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
- The `benchmark` command times Choice, Noul, and Score on one built-in text after warm-up. Use `--duration 10s` to set the measurement duration per type (10 seconds by default), `--type choice|noul|score|all` to select the types, and `--json` for machine-readable results. Run `localai_decisions benchmark --help` for the benchmark options.

## Choose a model by download size

Decision models use the GGUF model file format. The code after the colon selects a particular weight variant. The sizes below are for the file on disk. Leave additional memory for the running model and its input.

| Model for `--model` | Download | When to choose it |
| --- | ---: | --- |
| [`ggml-org/Kev-0.8B-GGUF:Q8_0`](https://huggingface.co/ggml-org/Kev-0.8B-GGUF/blob/main/Kev-0.8B-Q8_0.gguf) | 0.81 GB | Default and smallest choice. Start here for yes-or-no checks and routing. |
| [`ggml-org/Kev-4B-GGUF:Q4_K_M`](https://huggingface.co/ggml-org/Kev-4B-GGUF/blob/main/Kev-4B-Q4_K_M.gguf) | 3.03 GB | Larger Kev model to compare against the default when the labels are difficult to tell apart. |
| [`ggml-org/Bespoke-Nimble-9B-v3-GGUF:Q4_K_M`](https://huggingface.co/ggml-org/Bespoke-Nimble-9B-v3-GGUF/blob/main/Bespoke-Nimble-9B-v3-Q4_K_M.gguf) | 6.32 GB | An alternative decision model for a comparison on examples with known answers. |
| [`ggml-org/Clef-Flash-GGUF:Q8_0`](https://huggingface.co/ggml-org/Clef-Flash-GGUF/blob/main/Clef-Flash-Q8_0.gguf) | 9.66 GB | A larger choice for testing text decisions if the machine has room for the model. |
| [`ggml-org/OpenJev-GGUF:Q4_K_M`](https://huggingface.co/ggml-org/OpenJev-GGUF/blob/main/OpenJev-Q4_K_M.gguf) | 19.40 GB | The largest choice in this list. Evaluate it on a machine with enough free memory and disk space. |

The repository has run Kev-0.8B. The other choices use decision types supported by the tool. Compare them on a small set of examples before relying on their probabilities. Use only the text model file for Clef Flash and OpenJev.

## Check what is already available

Use read-only filesystem checks before setup. Look for `localai_decisions` on PATH or read the saved executable path at `<cgx data>/localai/localai_decisions.path`. Verify that the saved path points to an executable named `localai_decisions` under `<cgx data>/bins/`. If the saved path is absent, reuse a cached executable when you can verify it came from the selected Heidrun GitHub repository. If the executable and all model files are ready, run the executable directly from the user's project.

On macOS and Linux, the cgx cache is `${XDG_CACHE_HOME:-$HOME/.cache}/cgx` and `<cgx data>` is `${XDG_DATA_HOME:-$HOME/.local/share}/cgx`. cgx keeps downloaded source in its cache and build files and executables under `build/` and `bins/` in its data directory.

Use `HF_HUB_CACHE` for the Hugging Face hub cache when set, then `HUGGINGFACE_HUB_CACHE`. Otherwise use the `hub/` directory under `HF_HOME` or under `${XDG_CACHE_HOME:-$HOME/.cache}/huggingface`. Each repository has a folder named `models--owner--name`.

The default is `ggml-org/Kev-0.8B-GGUF:Q8_0`. In the cache directory for the selected model, read `refs/main` and verify that the file resolves in `snapshots/<revision>/`. The default file is `Kev-0.8B-Q8_0.gguf`. For another model use the repository name without `-GGUF`, followed by `-<variant>.gguf`. A user-supplied `.gguf` path can be used after checking that the file exists.

Treat incomplete or uncertain model caches as a possible download. Include any requested model update in the approval question because the update can download files. Do not use a cgx command for preflight if it could fetch or build the tool, even when the command would only print the executable path.

## Ask once before setup

If the tool needs a build, check for `cgx` on PATH or at `<cgx data>/runner/cgx`. Check for Git, a native compiler and linker, CMake, and a compatible Rust compiler and Cargo. With Rustup, use `rustup toolchain list` and `rustup run TOOLCHAIN rustc --version` to check an existing toolchain without updating it. If cgx is missing, check for curl, tar, and a checksum tool. Identify any other missing prerequisite and how it would be obtained before asking.

Ask **one** question and wait for explicit approval. List every required setup action, including downloads of cgx or prerequisites, the GitHub source fetch, Cargo dependency downloads, the build, and any model download. Give the relevant directories and the model name and download size when known. Explain both choices in the same question.

- **Approve** — Complete the listed setup and run the requested operation. One approval covers all listed steps.
- **Decline** — Leave setup untouched and explain which missing item prevents the requested operation.

If an unexpected requirement falls outside the approved steps, report the blocker and stop. Keep the user's existing default Rust toolchain intact.

## Set up and run with cgx

After approval, reuse cgx if available. Otherwise, select the published [cgx 0.2.0 archive](https://github.com/anelson/cgx/releases/tag/v0.2.0) for the operating system and processor. For example, Apple Silicon macOS uses `cgx-aarch64-apple-darwin.tar.xz`. Download the archive and its checksum into a temporary directory, verify the archive against the published checksum, and extract `cgx` to `<cgx data>/runner/cgx`. Invoke the runner by its absolute path. This keeps the bootstrap in the default cgx data directory. Remove the temporary archive after extraction.

Use an existing compatible Rust toolchain when available. If Rust is absent or incompatible and the user approved a separate installation, use the official [Rustup standalone installer](https://rust-lang.github.io/rustup/installation/index.html) with `--no-modify-path --profile minimal --default-toolchain 1.100.0`, `RUSTUP_HOME` under `<cgx data>/rustup`, and `CARGO_HOME` under `<cgx data>/cargo`. For the build subprocess, set the isolated `RUSTUP_HOME` and prepend the isolated Cargo `bin/` directory to its PATH. For a prerequisite without an approved installation method, report the blocker.

On macOS and Linux the commands below use cgx's default directories. Set `cgx` to the available executable path and `toolchain` to the installed Rustup toolchain name. For an existing Cargo installation without Rustup, omit the `+` argument and point `CARGO` at that Cargo executable. Cargo downloads go under `<cgx data>/cargo` for the skill's build. Use the same isolated `RUSTUP_HOME` on later builds if a separate Rust installation was needed. Set `GGML_METAL=OFF` for the processor build because cgx invokes Cargo from the workspace root.

```sh
cgx_data="${XDG_DATA_HOME:-$HOME/.local/share}/cgx"
tool_path=$(CARGO_HOME="$cgx_data/cargo" GGML_METAL=OFF "$cgx" "+$toolchain" \
    --git https://github.com/heidrun-org/localai_rust.git --branch main \
    --prebuilt-binary never --no-exec localai_decisions) || exit 1
test -x "$tool_path" || exit 1
mkdir -p "$cgx_data/localai"
printf '%s\n' "$tool_path" > "$cgx_data/localai/localai_decisions.path"
printf '%s\n' "$tool_path"
```

Run the printed executable from the user's project with the arguments for the request. Model downloads use the Hugging Face cache described above. For a later request, use the saved executable and cached model after checking both. Return actual output or the specific blocker.

For cleanup, list cached executables with `find "$cgx_data/bins" -type f -perm -100`. Remove the cgx cache and data directories when you want to discard their source, builds, tools, and Cargo downloads. The Hugging Face cache stores the model files separately.
