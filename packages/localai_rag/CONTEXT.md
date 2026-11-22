# Directory Context: `/packages/localai_rag`

## Purpose
Index and search named collections of local Markdown and plain-text documents.

## Key Exports & Entry Points
- `src/` contains the command line and indexing code. See its CONTEXT.md.
- Run the command with `pnpm --filter localai_rag cli`.

## Rules
- Use Local AI Embeddings for token limits and inference, and Local AI Vector Store for saved similarity search.

## Background
- https://github.com/heidrun-org/localai_rust/issues/27
