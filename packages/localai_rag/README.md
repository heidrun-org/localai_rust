# Local AI RAG

Local AI RAG searches a named collection of local documents by meaning. An agent can return the matching excerpts as a reading list or use the excerpts as evidence in an answer with citations. This second use is retrieval-augmented generation.

## Build and use

From this repository, build with `pnpm --filter localai_rag build`. Use the resulting `target/debug/localai_rag` binary, or substitute `pnpm --filter localai_rag cli` for `localai_rag` in the commands below.

```sh
localai_rag create --collection project-docs --root ./docs
localai_rag sync --collection project-docs
localai_rag list
localai_rag search --collection project-docs \
  --query "Why do worktree reviews use a separate server port?" \
  --limit 5 --chunks-per-document 2
```

All commands emit JSON. Collection names start with an ASCII letter or digit and can also contain hyphens and underscores. The source directory is stored as an absolute path. The default model is `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`. Set `--model` on creation to choose another model supported by [Local AI Embeddings](../localai_embeddings/README.md). The create command loads the model to establish its vector dimensions, downloading missing model files if needed. Inference runs locally. Each collection keeps the exact model revision selected at creation for future searches and refreshes. If an earlier collection has no saved model revision, run `sync` once to rebuild the index and pin the current revision.

The default collection directory is `heidrun-localai-rust/rag-collections` under the current user's local application data directory. Use `LOCALAI_RAG_HOME` or `--data-directory` to choose a different directory.

## Refresh and search

Run `sync` after creating, editing, or deleting files. The command recursively indexes UTF-8 `.md`, `.markdown`, and `.txt` files, skips symbolic links and empty files, reuses unchanged embeddings, and deletes stale chunks. The refresh counters report the work performed. A failed refresh leaves the previous complete index and its refresh time searchable. Search uses the saved collection across runs and opens the index for reading only.

Each search result includes a document identifier, absolute source path, SHA-256 content hash, and similarity score. Documents rank by their best chunk, and the document limit applies after grouping chunks. Every chunk includes the exact text, its score, byte offsets (zero-based, end excluded), and line numbers (one-based, both endpoints included). Markdown chunks also carry their heading hierarchy, block type, and any table column names, explicit caption, and data row range. Put `Table: <title>` on the line immediately above a pipe table to record a caption.

To cite a result, use the returned source path and line numbers and quote or refer to the exact returned text. For a citation to a file that may have changed since indexing, compare its current SHA-256 hash to the returned content hash or refresh before citing. The scores express cosine similarity. Search can return weak matches because there is no minimum score. Read each excerpt for actual support, identify tentative matches as such, and when none supports the query, tell the user that the collection yielded no relevant evidence.
