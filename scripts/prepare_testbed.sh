#!/bin/sh
# Create a standalone test project and print its absolute path on standard output.
# The caller launches either Codex or Claude Code from that directory.
set -eu

# Resolve paths from this script so it can be called from any working directory.
# Clear CDPATH so a user's shell configuration cannot add output to the path.
repository="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
testbed="$repository/testbed_folders"

# Use a new directory for every run. The test should start outside the checkout
# so the agent cannot accidentally discover the source through parent folders.
workspace="$(mktemp -d "${TMPDIR:-/tmp}/localai-testbed.XXXXXX")"

# Copy only the distributable skill files. This tests what an end user
# installs and makes an accidental dependency on a companion file visible.
for skill in localai_embeddings localai_decisions localai_llm localai_vector_store localai_rag; do
    mkdir -p "$workspace/skills/$skill"
    cp "$repository/skills/$skill/SKILL.md" "$workspace/skills/$skill/"
done

# Codex and Claude Code discover project skills at different paths. Each path
# points to the same copied skill tree. The relative links stay valid if the
# standalone directory is moved.
for agent in .agents .claude; do
    mkdir "$workspace/$agent"
    ln -s ../skills "$workspace/$agent/skills"
done

# Both agents follow the same instructions. Copy the common instructions once
# and expose them to Claude Code under the filename Claude Code reads.
cp "$testbed/AGENTS.md" "$testbed/README.md" "$workspace/"
ln -s AGENTS.md "$workspace/CLAUDE.md"

# Keep stdout to one line so a shell can capture the path. Preparing the
# workspace does not invoke either agent or install tools or model files.
printf '%s\n' "$workspace"
