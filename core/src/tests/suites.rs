//! runs the same suites the gui buttons run, so ci fails when one regresses.

use super::{run_gpu_tests, run_tests};

fn assert_suite_passes(name: &str, lines: &[String]) {
    let failed: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("N "))
        .map(String::as_str)
        .collect();
    assert!(
        failed.is_empty(),
        "{name} suite has failing tests:\n{}",
        failed.join("\n")
    );
    assert!(
        lines.iter().any(|l| l.starts_with("Y ")),
        "{name} suite ran no tests"
    );
}

// the cpu suite reports a possible first-test timeout on macOS jit cold start
#[test]
#[cfg_attr(target_os = "macos", ignore = "jit cold start can time out")]
fn cpu_suite() {
    assert_suite_passes("cpu", &run_tests());
}

#[test]
fn gpu_suite() {
    assert_suite_passes("gpu", &run_gpu_tests());
}
