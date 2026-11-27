# Local AI embeddings

This package runs on the computer with no network call after the first download of the model.

`localai_embeddings` computes embeddings on your computer with a Rust library and command line tool. It accepts ordinary text or source code as text. Input text stays on your computer. This version handles text only; code is accepted as text, but the default model was trained for natural language rather than code search.

## Requirements

- Rust and Cargo.
- A CPU for the default backend.
- Internet access on the first run to download the model files.

The default model is `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2`, which produces 384-dimensional vectors and supports English, French, and other languages. Its files occupy approximately 460 MiB in the standard Hugging Face cache, normally `~/.cache/huggingface/hub`. Hugging Face cache environment variables can change that location.

## Compute an embedding

From the repository root, pass text, a file, or a pipe:

```bash
pnpm --filter localai_embeddings cli --text "Bonjour, ceci est un exemple."
pnpm --filter localai_embeddings cli --file README.md
cat ./notes.txt | pnpm --filter localai_embeddings cli
```

The package command runs inside the package folder: `--file README.md` reads this package's README. Use an absolute path for a file elsewhere. With no text, file, or pipe, the command prompts for pasted text and waits for Control-D.

Files must contain UTF-8 text and are read into memory. Binary documents need text extraction before use.

The JSON output contains `model`, `backend`, `dimensions`, and `documents`. Each document contains its source name, one `global_vector`, and `chunks`. Each chunk contains its source byte offsets, token count, text, and vector. Use `--output result.json` to save the output.

The `pnpm` command uses an optimized release build. The first invocation compiles the program; later invocations reuse the compiled program. Add `--verbose` to show input reading, timings for each model-loading stage, and embedding timings on standard error:

```bash
pnpm --filter localai_embeddings cli --verbose --file README.md --output /private/tmp/localai-embeddings.json
```

Compare the global vectors of exactly two inputs:

```bash
pnpm --filter localai_embeddings cli --compare \
  --text "The cat sleeps on the chair." \
  --text "A cat is resting on a seat."
```

## Long documents and chunking

The command embeds the whole document by splitting the document into chunks that fit the selected model. The default model uses its Sentence Transformers limit of **128 tokens**, including special tokens. The model's underlying BERT architecture supports 512 positions, but that architecture limit is not the preferred sentence embedding length. Other supported models use their own Sentence Transformers setting when present, bounded by the architecture limit.

`--chunking auto` chooses Markdown for `.md` files, code for source files such as `.rs`, and lines for `.log` or `.jsonl` files. Other inputs use ordinary text. Override the choice with `--chunking text`, `markdown`, `code`, or `lines` (`chat` is an alias for `lines`).

Markdown uses headings, paragraphs, and fenced blocks; ordinary text uses paragraphs and sentences; code uses blocks and lines; chat uses lines. Adjacent blocks are packed together when they fit. Oversized blocks are split further, with tokenizer boundaries as the final fallback. The code mode does not parse the programming language's syntax.

Overlap defaults to **zero**. Request a maximum repeated tail of the previous chunk with:

```bash
pnpm --filter localai_embeddings cli --file README.md --overlap-tokens 16
```

Overlap reserves space inside each chunk's token limit and can be smaller than requested near structural boundaries. Every chunk receives a normalized vector. The global vector is a normalized mean of the chunk vectors, weighted by content token count. The global vector is an approximation for document comparison; chunk vectors retain more local detail. Overlapped text contributes to more than one chunk and therefore also influences the mean more than once.

Rust callers use `EmbeddingEngine::embed_document` for long documents. The lower-level `EmbeddingEngine::embed` handles short texts and rejects inputs above the model limit instead of silently discarding the end.

## Model cache and updates

By default, model loading uses cached files immediately without checking Hugging Face for newer versions. Files missing from the cache are downloaded automatically. The cache also remembers when an optional file does not exist, so later runs do not repeat that lookup.

Use `--check-model-updates` to ask Hugging Face to check the current model files and download newer versions when available:

```bash
pnpm --filter localai_embeddings cli --check-model-updates --verbose --file README.md
```

Rust callers select the same behavior with `EmbeddingConfig::check_model_updates`, which defaults to `false`.

## Select a model and backend

Choose another supported Hugging Face model with `--model <model-id>`. This version supports BERT sentence embedding models with `model.safetensors` weights and mean pooling. A present Sentence Transformers pooling configuration must specify mean pooling only. If that optional file is absent, the tool uses mean pooling. Unsupported architectures produce an error.

The default backend is `cpu`. The CPU worker count is selected automatically by Candle; on Apple Silicon, Candle normally selects the performance core count. Set `--threads 10` to request ten CPU workers. More workers do not guarantee more throughput, especially when some cores are slower or other processes are active.

`--backend metal` requires a macOS build with the `metal` feature. `--backend cuda` requires the `cuda` feature and a compatible NVIDIA GPU. Run these commands from the repository root:

```bash
pnpm --filter localai_embeddings exec cargo run --release --features metal -- --backend metal --file README.md
pnpm --filter localai_embeddings exec cargo run --release --features cuda -- --backend cuda --file README.md
```

The native command reports that `wasm` requires a separate browser build. An unavailable backend produces an error rather than silently selecting another backend.

## Benchmark caller-supplied input

Use `--benchmark` to measure embedding throughput. The command loads the model, runs one warm-up pass, then repeatedly embeds the supplied text for `--duration <time>`. The default is `10s`. When the window expires, the current pass finishes. The timed measurement includes chunking, tokenization, chunk inference, and whole-text embedding computation.

The primary result, `input_mib_per_second`, is the number of mebibytes of original text processed per measured second. A mebibyte is 1,048,576 bytes. Each byte of the original input counts once per completed pass, even with overlapping chunks. The report also gives the completed pass count, total input bytes, actual elapsed time, and rates for chunk embeddings, chunk tokens, and padded model positions.

Use the same text and chunking options for every model you compare. For example, run a five-second benchmark on a file from the repository root:

```bash
pnpm --filter localai_embeddings cli --benchmark --duration 5s --file README.md
```
