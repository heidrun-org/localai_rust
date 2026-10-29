// Checks that the feature `metal` and the environment variable GGML_METAL agree.

fn main() {
    println!("cargo:rerun-if-env-changed=GGML_METAL");

    let metal_feature = std::env::var("CARGO_FEATURE_METAL").is_ok();
    let metal_variable = std::env::var("GGML_METAL").unwrap_or_default();
    if metal_feature && metal_variable != "ON" {
        panic!("The feature `metal` needs the environment variable GGML_METAL=ON, for example: GGML_METAL=ON cargo build --release --features metal");
    }
}
