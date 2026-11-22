# Directory Context: `/testbed_folders`

## Purpose
Shared test workspace for exercising the Local AI skills in Codex and Claude Code.

## Key Exports & Entry Points
- `sh scripts/prepare_testbed.sh` prepares a standalone workspace and prints its path.

## Rules
- The installed skills are copied from the repository's top-level `skills/` directory.
- First-run setup and model files live outside the repository.

## Background
- https://github.com/heidrun-org/localai_rust/issues/18
