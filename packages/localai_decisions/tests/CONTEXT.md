# Directory Context: `/packages/localai_decisions/tests`

## Purpose
This folder holds the Rust tests of the package, and the answers of the server of llama.cpp that the tests compare with.

## Key Exports & Entry Points
- `server_reference.rs`: compares the answers and the number of prompt tokens with the answers of the server, on the requests of `fixtures/requests.json`.
- `requests_and_errors.rs`: the errors of a request, the batch size, the refused GPU, and the model names.
- `benchmark_command.rs`: benchmark discovery, help, decision-count, device validation, and the backend alias without loading weights.
- `command_line.rs`: runs the compiled command line tool.
- `fixtures/`: `requests.json` (the requests) and `server_reference.json` (the answers of the server).
- Command to run the fast tests: `pnpm --filter localai_decisions test`
- Command to run all the tests, also the slow tests: `pnpm --filter localai_decisions test:full`

## Rules
- The tests need the model ggml-org/Kev-0.8B-GGUF (file `Q8_0`) in the Hugging Face cache, and the first run downloads it (0.81 GB).
- Never edit `server_reference.json` by hand; run the script in `server_reference/`.
- A new request goes into `requests.json`, then the script writes its server answers.
- A test that compares the answers of the model, or that starts the command line tool with the model, is marked `#[ignore = "slow: ..."]`, so that `test` stays fast; `test:full` runs it.

## Background
- The fast and slow split of the tests comes from https://github.com/heidrun-org/localai_rust/issues/10.
- The tolerance of the comparison is in `server_reference.rs`; the measured largest difference is in the README of the package.
