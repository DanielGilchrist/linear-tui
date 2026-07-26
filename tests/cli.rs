use std::process::Command;

#[test]
fn headless_detail_render_includes_the_cached_markdown_bodies() {
    let output = Command::new(env!("CARGO_BIN_EXE_linear-tui"))
        .args([
            "render",
            "--detail",
            "DAN2-7",
            "--fixture",
            "fixtures/dans-donuts.json",
            "--width",
            "100",
            "--height",
            "40",
        ])
        .output()
        .expect("run the headless render subcommand");

    assert!(
        output.status.success(),
        "render exited with failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Suspect"),
        "detail render should include the markdown description body, got:\n{stdout}"
    );
    assert!(
        stdout.contains("thermocouple"),
        "detail render should include a markdown comment body, got:\n{stdout}"
    );
    assert!(
        stdout.contains("🚀"),
        "detail render should include the issue's reaction chips, got:\n{stdout}"
    );
}
