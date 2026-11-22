# Directory Context: `/skills`

## Purpose
Distributable agent skills for the Heidrun Local AI Rust commands.

## Key Exports & Entry Points
- `localai_embeddings/` computes embeddings for text and files.
- `localai_decisions/` runs yes-or-no and multiple-choice decisions.
- `localai_llm/` generates and summarizes text.
- `localai_rag/` searches local Markdown and plain-text collections and cites passages.
- `localai_vector_store/` creates, populates, and searches local vector stores.

## Rules
- Each SKILL.md contains the cgx setup, usage instructions, and any model choices for its tool.
- Each skill obtains one explicit approval for all required setup and any model downloads.

## Background
- https://github.com/heidrun-org/localai_rust/issues/18
- https://github.com/heidrun-org/localai_rust/issues/22
