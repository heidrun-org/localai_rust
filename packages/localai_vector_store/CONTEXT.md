# Directory Context: `/packages/localai_vector_store`

## Purpose
Store and search embeddings on the user's computer.

## Key Exports & Entry Points
- `src/` provides the Rust library and command line tool. See its CONTEXT.md.
- `tests/` verifies the public entry points. See its CONTEXT.md.
- Run the package tests with `pnpm --filter localai_vector_store test`.

## Rules
- The package accepts precomputed embeddings and imports no other workspace package.

## Background
- https://github.com/heidrun-org/localai_rust/issues/20
