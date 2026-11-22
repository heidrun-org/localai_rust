# Directory Context: `/skills`

## Purpose
Distributable agent skills for the three Heidrun Local AI Rust commands.

## Key Exports & Entry Points
- `localai_embeddings/` computes embeddings for text and files.
- `localai_decisions/` runs yes-or-no and multiple-choice decisions.
- `localai_llm/` generates and summarizes text.

## Rules
- Each SKILL.md contains the cgx setup, model choices, and usage instructions for its tool.
- Each skill obtains one explicit approval for all required setup and model downloads.

## Background
- https://github.com/heidrun-org/localai_rust/issues/18
- https://github.com/heidrun-org/localai_rust/issues/22
