// Builds the C wrapper of the Clef decision order.

fn main() {
    println!("cargo:rerun-if-changed=src/clef_decision_order.cpp");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("src/clef_decision_order.cpp")
        .compile("clef_decision_order");
}
