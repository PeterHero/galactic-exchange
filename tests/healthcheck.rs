mod common;
use common::setup;

#[test]
fn healthcheck() {
    let setup = setup();

    // Query container
    let response = ureq::get(format!("{}/health", setup.base_url)).call();

    assert!(response.is_ok(), "GET /health response should be 200");
}
