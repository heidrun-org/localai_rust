# Directory Context: `/skills`

## Purpose
Distributable agent skills for the three Heidrun Local AI Rust commands.

## Key Exports & Entry Points
- `localai_embeddings/` computes embeddings for text and files.
- `localai_decisions/` runs yes-or-no and multiple-choice decisions.
- `localai_llm/` generates and summarizes text.

## Rules
- Each SKILL.md contains everything the agent needs to install mise, build its tool, and run it.
- Each skill asks for explicit approval before setup or model downloads.

## Background
- https://github.com/heidrun-org/localai_rust/issues/18
