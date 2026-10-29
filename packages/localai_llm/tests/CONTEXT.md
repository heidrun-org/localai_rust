# Directory Context: `/packages/localai_llm/tests`

## Purpose
This folder holds the Rust tests that run the compiled command line tool.

## Key Exports & Entry Points
- `command_line.rs`: the prompt from the option and from standard input, the JSON result, the largest number of tokens, the output file, and the errors.
- `benchmark_command.rs`: benchmark discovery, help, count validation, device validation, and the backend alias without loading weights.
- Command to run the fast tests: `pnpm --filter localai_llm test`
- Command to run all the tests, also the slow tests: `pnpm --filter localai_llm test:full`

## Rules
- The slow tests need the model LiquidAI/LFM2.5-1.2B-Instruct-GGUF (file `Q4_K_M`) in the Hugging Face cache, and the first run downloads it (0.73 GB).
- A test that starts the command line tool with the model is marked `#[ignore = "slow: ..."]`, so that `test` stays fast; `test:full` runs it.
- A slow test checks a fact that does not change with the exact words of the model, for example "Paris" in the answer, the stop reason, or the token count.
