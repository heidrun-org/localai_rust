# Local AI Vector Store

`localai_vector_store` stores precomputed embeddings in SQLite and ranks them by cosine similarity. Use the Rust library for precomputed vectors or the command line tool for JSON output from [localai_embeddings](../localai_embeddings/README.md).

## Create and search a store

Build the tools from their package folders with `cargo build --locked --release`, then put the resulting executables on your command search path. Run the following example from a directory outside the repository. The embedding command may need to download the model on first use.

```sh
printf '%s\n' 'The library opens at nine every morning.' > library.txt
printf '%s\n' 'Tomatoes need plenty of sunlight to grow.' > garden.txt
localai_embeddings --file library.txt --file garden.txt --output documents.json
```

Read `model` and `dimensions` from `documents.json`. The example uses the default Hugging Face model. When you know the exact model revision, supply it with `--model-revision` on `create`, `add`, and `search`.

```sh
localai_vector_store create --parent-dir ./stores \
  --description "Library and garden notes" --model-provider huggingface \
  --model sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 --dimensions 384
```

Copy the `identifier` from the output into `store_id`.

```sh
store_id='<identifier from create>'
localai_vector_store add --parent-dir ./stores --store "$store_id" \
  --input documents.json --model-provider huggingface

localai_embeddings --text 'When does the library open?' --output query.json
localai_vector_store search --parent-dir ./stores --store "$store_id" \
  --query query.json --model-provider huggingface --limit 1
```

The search command reopens the store and returns `library.txt` as the closest source. Each result includes the embedding identifier, source, and score. The document embeddings remain available for later queries.

## Command line options

All commands accept `--parent-dir PATH`. Without it, the parent directory is `heidrun-localai-rust/vector-stores` under the operating system's per-user application data folder. Each store has a generated identifier and a `store.sqlite3` database in the folder named after the identifier. Commands print JSON.

| Command | Options |
| --- | --- |
| `create` | `--description TEXT --model-provider PROVIDER --model MODEL --dimensions N [--model-revision REVISION]` |
| `list` | Lists stores in the selected parent directory. |
| `describe` | `--store ID` |
| `add` | `--store ID --input FILE --model-provider PROVIDER [--model-revision REVISION] [--vectors document\|chunk\|both] [--source-prefix PREFIX]` |
| `search` | `--store ID --query FILE --model-provider PROVIDER [--model-revision REVISION] [--limit N]` |

`add` saves document vectors by default. Use `--vectors chunk` for individual chunks or `--vectors both` for both kinds. Identifiers use the source name and the chunk position, if any. Use `--source-prefix` to distinguish identical source names from separate projects or separate text inputs. If any identifier is already present, the whole import fails and saved embeddings are preserved.

`search` uses the document vector from a query file with exactly one document and returns up to ten results by default. A query or import must declare the same provider and full model identifier as the store. Omitted revisions match when both the store and the request omit the revision. Every vector must have the store's declared number of values and a finite, nonzero length.

## Rust library

Use `Store::create`, `Store::open`, `Store::add`, and `Store::search` with an `EmbeddingModel`. Callers supply their own embedding identifiers and source references when using the library. Multiple embeddings can refer to the same source.

## Tests

Run `pnpm --filter localai_vector_store test` at the repository root.
