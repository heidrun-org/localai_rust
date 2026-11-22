# Directory Context: `/packages/localai_vector_store/tests`

## Purpose
Verify the command line and SQLite persistence through the public entry points.

## Key Exports & Entry Points
- Run the tests with `pnpm --filter localai_vector_store test`.

## Rules
- Tests use temporary stores and require no embedding model downloads.

## Background
- https://github.com/heidrun-org/localai_rust/issues/20
