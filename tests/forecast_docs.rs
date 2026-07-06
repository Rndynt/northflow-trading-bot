use std::fs;

#[test]
fn implementation_report_has_exact_command_results_section() {
    let content = fs::read_to_string("docs/forecast-ml-implementation-report.md")
        .expect("docs/forecast-ml-implementation-report.md should exist");
    assert!(
        content.contains("## Command results"),
        "implementation report must contain a '## Command results' section"
    );
    assert!(
        !content.contains("See final agent response"),
        "implementation report must not defer command results to the final agent response"
    );
    for cmd in [
        "cargo fmt --check",
        "cargo test",
        "cargo run -- forecast --config config/forecast.toml",
        "cargo run -- research --config config/research.toml",
    ] {
        assert!(
            content.contains(cmd),
            "implementation report must document the exact outcome of `{cmd}`"
        );
    }
}
