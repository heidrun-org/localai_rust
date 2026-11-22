# Directory Context: `/packages/localai_rag/src`

## Purpose
Parse local documents, preserve source locations, and maintain and search saved collections.

## Key Exports & Entry Points
- `localai_rag` provides the create, sync, list, and search commands.

## Rules
- A collection refresh publishes its chunk changes and refresh time together. Searches and refreshes use the saved model revision.
- Search reads saved chunks and ranks documents by their highest matching chunk score.
- Chunk ranges refer to the original file bytes even when embedding input includes added Markdown context.

## Background
- https://github.com/heidrun-org/localai_rust/issues/27
