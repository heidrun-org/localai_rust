//! Runs the compiled command line tool.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn tool() -> Command {
    Command::new(env!("CARGO_BIN_EXE_localai_llm"))
}

fn json_output(output: std::process::Output) -> Value {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
#[ignore = "slow: run it with pnpm --filter localai_llm test:full"]
fn the_model_answers_a_simple_question() {
    let output = tool()
        .args(["--prompt", "What is the capital of France? Answer with one word.", "--temperature", "0", "--json"])
        .output()
        .unwrap();
    let result = json_output(output);
    assert!(result["text"].as_str().unwrap().contains("Paris"), "{result}");
    assert_eq!(result["model"], "LiquidAI/LFM2.5-1.2B-Instruct-GGUF");
    assert_eq!(result["stop_reason"], "end");
    assert!(result["prompt_tokens"].as_u64().unwrap() > 5);
}

#[test]
#[ignore = "slow: run it with pnpm --filter localai_llm test:full"]
fn a_prompt_comes_from_the_option_or_from_standard_input_with_the_same_text() {
    let prompt = "Name one color.";
    let from_option = json_output(tool().args(["--prompt", prompt, "--temperature", "0", "--json"]).output().unwrap());

    let mut child = tool()
        .args(["--temperature", "0", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(prompt.as_bytes()).unwrap();
    let from_stdin = json_output(child.wait_with_output().unwrap());
    assert_eq!(from_option["text"], from_stdin["text"]);
}

#[test]
#[ignore = "slow: run it with pnpm --filter localai_llm test:full"]
fn the_generation_stops_at_the_largest_number_of_tokens() {
    let output = tool()
        .args(["--prompt", "Count from 1 to 100.", "--max-tokens", "5", "--json"])
        .output()
        .unwrap();
    let result = json_output(output);
    assert_eq!(result["generated_tokens"], 5);
    assert_eq!(result["stop_reason"], "max_tokens");
}

#[test]
#[ignore = "slow: run it with pnpm --filter localai_llm test:full"]
fn the_text_is_written_while_it_grows_and_to_a_file() {
    let streamed = tool().args(["--prompt", "Say hello.", "--temperature", "0"]).output().unwrap();
    assert!(streamed.status.success(), "{}", String::from_utf8_lossy(&streamed.stderr));
    let streamed = String::from_utf8(streamed.stdout).unwrap();

    let path = std::env::temp_dir().join(format!("localai_llm_output_{}.txt", std::process::id()));
    let output = tool().args(["--prompt", "Say hello.", "--temperature", "0", "--output"]).arg(&path).output().unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let from_file = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(streamed, from_file);
    assert!(streamed.trim().is_empty() == false);
}

#[test]
fn an_error_is_written_on_standard_error_with_a_non_zero_code() {
    let output = tool().args(["--model", "/no/such/model.gguf", "--prompt", "hello"]).output().unwrap();
    assert_eq!(output.status.success(), false);
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.starts_with("Error: The model file /no/such/model.gguf does not exist"), "{message}");
}

#[test]
fn an_empty_prompt_is_refused_before_the_model_loads() {
    let output = tool().args(["--prompt", "  "]).output().unwrap();
    assert_eq!(output.status.success(), false);
    assert!(String::from_utf8_lossy(&output.stderr).contains("The prompt is empty"));

    let output = tool().stdin(Stdio::null()).output().unwrap();
    assert_eq!(output.status.success(), false);
    assert!(String::from_utf8_lossy(&output.stderr).contains("No prompt"));
}

#[test]
fn a_gpu_in_a_build_without_a_gpu_feature_is_refused() {
    if cfg!(any(feature = "metal", feature = "cuda", feature = "vulkan")) {
        return;
    }
    let output = tool().args(["--backend", "gpu", "--prompt", "hello"]).output().unwrap();
    assert_eq!(output.status.success(), false);
    assert!(String::from_utf8_lossy(&output.stderr).contains("This build has no GPU backend"));
}

#[test]
fn a_model_name_without_an_owner_is_refused() {
    let output = tool().args(["--model", "no-such-model", "--prompt", "hello"]).output().unwrap();
    assert_eq!(output.status.success(), false);
    assert!(String::from_utf8_lossy(&output.stderr).contains("of the form `owner/name`"));
}

#[test]
fn the_system_message_and_the_raw_prompt_cannot_go_together() {
    let output = tool().args(["--raw", "--system", "Be short.", "--prompt", "hello"]).output().unwrap();
    assert_eq!(output.status.success(), false);
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
}
