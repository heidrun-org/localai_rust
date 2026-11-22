# Directory Context: `/packages/localai_vector_store/src`

## Purpose
Maintain SQLite vector stores and adapt command line requests to the storage library.

## Key Exports & Entry Points
- `Store` creates, opens, renames, describes, and searches a local store. It also parses and formats store identifiers.
- `Store::applyRefresh` atomically removes and adds embeddings and records refresh time.
- `Store::replaceUnpinnedEmbeddings` atomically replaces an unpinned index and records the new model revision.
- `Store::openReadOnly` opens a saved store without write access.
- `localai_vector_store` reads embeddings from JSON input and returns JSON output.

## Rules
- Each store lives in its own folder and uses a single embedding model configuration.
- Store commands use the identifier saved in the database and support both original and readable folder names.
- New store folder names and formatted store identifiers use underscores as separators.
- The library checks declared model identity and vector length on every insert and query.
- A batch insert succeeds in full or leaves the saved embeddings untouched.

## Background
- https://github.com/heidrun-org/localai_rust/issues/20
