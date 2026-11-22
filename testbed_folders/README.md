# Test the Local AI skills

From the repository root, run `sh scripts/prepare_testbed.sh` separately for Codex and Claude Code. The command copies the three distributable skill files and the test workspace instructions. Start the agent in the printed directory. Use an environment where the Local AI tools, cgx, and other copies of the skills are absent. Use an existing compatible Rust toolchain, platform compiler, linker, and CMake for the decision and text generation tools.

Set these variables in the shell that starts the agent. cgx uses its regular layout inside the disposable cache and data roots. The skills keep Cargo downloads inside cgx's data directory.

```sh
export XDG_CACHE_HOME="$PWD/cache"
export XDG_DATA_HOME="$PWD/data"
export XDG_CONFIG_HOME="$PWD/config"
export HF_HOME="$PWD/huggingface"
export HF_HUB_CACHE="$HF_HOME/hub"
export HF_XET_CACHE="$HF_HOME/xet"
```

Use a new workspace for any test that must start with cgx, executable, and model caches empty.

Run the following requests in each agent.

| Skill | Request | Check |
| --- | --- | --- |
| Embeddings | Compute a local embedding for "The library opens at nine." and save it in embedding.json. | The JSON has a global vector of 384 values. |
| Decisions | Use the local decision model. I was charged twice. Which team should help, billing, shipping, or technical? | The tool returns an option and its probability. |
| Text generation | Use the local language model to summarize this in one sentence. The library opens at noon on Friday because staff have training in the morning. | The generated text mentions the Friday noon opening. |

1. Make a request and check that the agent asks one question before setup. The question lists all required actions and storage locations, the missing model and size, and what approval or refusal means.
2. Decline and verify that the runner, cgx cache and data directories, and model files remain absent.
3. Make the request again and approve the listed steps. Check that cgx fetches the named package from GitHub, compiles it, and runs the request. Record the actual command output and executable path.
4. Repeat the request and verify that the agent runs the cached executable with the cached model without a setup question.

Record the agent version and platform. For a first GitHub check, run the executable with `--help` to test the fetch and build alone. Use the request table to verify the later model runs.

## Verified on Apple Silicon macOS

The standalone GitHub check used cgx 0.2.0 and Rust 1.100.0. It fetched `heidrun-org/localai_rust`, built `localai_embeddings`, and returned this help output.

```text
Compute text embeddings locally

Usage: localai_embeddings [OPTIONS]
```

cgx created the Git checkout under `$XDG_CACHE_HOME/cgx/git-checkouts/`, the build under `$XDG_DATA_HOME/cgx/build/`, and the executable under `$XDG_DATA_HOME/cgx/bins/localai_embeddings-0.3.5/`. The three tools were then built and run with fresh model files in a disposable cache. The embedding contained 384 values. The decision selected billing with probability 0.8706. The generated summary retained the Friday noon opening.

Codex 0.171.0 and Claude Code 2.1.326 each asked one setup question for each skill in separate fresh workspaces. The setup directories and model files stayed absent after refusal. With the verified executables and model files in place, both agents ran all three requests without a setup question.
