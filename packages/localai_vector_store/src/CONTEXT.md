# Directory Context: `/packages/localai_vector_store/src`

## Purpose
Maintain SQLite vector stores and adapt command line requests to the storage library.

## Key Exports & Entry Points
- `Store` creates, opens, describes, and searches a local store.
- `localai_vector_store` reads embeddings from JSON input and returns JSON output.

## Rules
- Each store lives in its own folder and uses a single embedding model configuration.
- The library checks declared model identity and vector length on every insert and query.
- A batch insert succeeds in full or leaves the saved embeddings untouched.

## Background
- https://github.com/heidrun-org/localai_rust/issues/20
