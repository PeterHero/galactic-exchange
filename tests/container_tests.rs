use std::{process::Command, thread, time::Duration};

#[test]
fn healthcheck() {
    // Build image
    let status = Command::new("docker")
        .args(["build", "-t", "galactic-exchange:test", "."])
        .status()
        .expect("failed to execute docker cmd");

    assert!(status.success(), "build failed");

    // Run container
    let output = Command::new("docker")
        .args(["run", "-d", "-p", "8080:8080", "galactic-exchange:test"])
        .output()
        .expect("failed to execute docker cmd");

    assert!(output.status.success(), "run failed");

    let container_id = String::from_utf8(output.stdout).unwrap().trim().to_string();

    // Wait for startup
    thread::sleep(Duration::from_secs(2));

    // Query container
    let response = ureq::get("http://localhost:8080/health").call();

    assert!(response.is_ok(), "GET /health response should be 200");

    // Cleanup
    Command::new("docker")
        .args(["rm", "-f", &container_id])
        .status()
        .unwrap();
}
