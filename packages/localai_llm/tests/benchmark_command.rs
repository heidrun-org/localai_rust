//! Checks benchmark command discovery and validation without loading model weights.
use std::process::Command;

fn tool() -> Command {
    Command::new(env!("CARGO_BIN_EXE_localai_llm"))
}

#[test]
fn benchmark_is_a_discoverable_subcommand() {
    let output = tool().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("benchmark"));
    let output = tool().args(["benchmark", "--help"]).output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for option in ["--model", "--generations", "--max-tokens", "--output", "--json", "--device", "--backend"] {
        assert!(help.contains(option), "{help}");
    }
    assert!(help.contains("[default: 3]"));
}

#[test]
fn invalid_benchmark_counts_and_device_fail_before_model_loading() {
    for args in [
        vec!["benchmark", "--generations", "0"],
        vec!["benchmark", "--generations", "-1"],
        vec!["benchmark", "--max-tokens", "0"],
        vec!["benchmark", "--device", "unknown"],
        vec!["benchmark", "--backend", "unknown"],
        vec!["benchmark", "--device", "cpu", "--backend", "gpu"],
    ] {
        let output = tool().args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("before timing"), "{error}");
    }
}

#[test]
fn model_and_backend_options_are_honored_before_and_after_benchmark() {
    for arguments in [
        vec!["--model", "no-such-model", "benchmark"],
        vec!["benchmark", "--model", "no-such-model"],
        vec!["--backend", "cpu", "benchmark", "--model", "no-such-model"],
        vec!["benchmark", "--backend", "cpu", "--model", "no-such-model"],
    ] {
        let output = tool().args(&arguments).output().unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Loading no-such-model on cpu"), "{error}");
        assert!(!error.contains("unexpected argument"), "{error}");
    }
}
