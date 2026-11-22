# Test the Local AI skills

From the repository root, run `sh scripts/prepare_testbed.sh` separately for Codex and Claude Code. The preparation command copies only the three SKILL.md files and one instruction file into a temporary workspace. Both agents link to that skill tree, and `CLAUDE.md` links to `AGENTS.md`. Start the agent inside that workspace, with a fresh setup directory and model cache. For example, from the printed directory:

```sh
export LOCALAI_SETUP_DIR="$PWD/setup"
export HF_HUB_CACHE="$PWD/model-cache"
```

Use an environment without the Local AI executables on PATH, and without any same-named user-installed skills or accessible source checkout. Start one run per agent without mise available to test the bootstrap prompt. CMake and the platform compiler should be available for the decision and generation builds unless you are testing missing prerequisites.

Use these requests in each agent:

| Skill | Request | Check |
| --- | --- | --- |
| Embeddings | Compute a local embedding for "The library opens at nine." and save it in embedding.json. | The JSON has a global vector of 384 values. |
| Decisions | Use the local decision model. I was charged twice. Which team should help, billing, shipping, or technical? | The tool returns an option and its probability. |
| Text generation | Use the local language model to summarize this in one sentence. The library opens at noon on Friday because staff have training in the morning. | The generated text mentions the Friday noon opening. |

For each skill, decline its initial setup proposal and check that the proposed package, model, and missing tools were not installed. Ask again and approve the needed steps. Record the agent's prompt, the executable path, and a short excerpt of the real output. Repeat the request and check that the executable and model are reused without a setup prompt. Record the agent version and platform. A tool installation or a help screen alone does not verify the model run.

## Verified on Apple Silicon macOS

With mise 2026.11.12 and managed Rust 1.100.0, the documented setup commands installed all three executables in a fresh standalone workspace. A run from an empty model cache returned:

```text
Vector length: 384
Decision: billing 0.8706388528570187
Summary: The library will open at noon on Friday after staff completed morning training.
```

Codex 0.171.0 and Claude Code 2.1.326 both asked for approval before setting up embeddings and left the setup and model cache absent after refusal. Both agents ran all three commands with the executables and model cache in place. The clean installs and model downloads were run directly from the workspace using the documented commands.
