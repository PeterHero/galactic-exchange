mod common;

use std::collections::HashMap;

use common::setup;
use galactic_exchange::{FieldName, FieldValue, serialize_message};
use ureq::{
    Body, Error,
    http::{Response, StatusCode},
};

fn register(url: &str, username: &str, password: &str) -> Result<Response<Body>, Error> {
    let body: HashMap<FieldName, FieldValue> = [
        ("username".into(), String::from(username).into()),
        ("password".into(), String::from(password).into()),
    ]
    .into();

    let message = serialize_message(body).unwrap();

    ureq::post(format!("{}/register", url))
        .header("Accept", "application/x-galacticbuf")
        .send(message)
}

fn login(url: &str, username: &str, password: &str) -> Result<Response<Body>, Error> {
    let body: HashMap<FieldName, FieldValue> = [
        ("username".into(), String::from(username).into()),
        ("password".into(), String::from(password).into()),
    ]
    .into();

    let message = serialize_message(body).unwrap();

    ureq::post(format!("{}/login", url))
        .header("Accept", "application/x-galacticbuf")
        .send(message)
}

#[test]
fn register_successful() {
    let setup = setup();

    let response = register(&setup.base_url, "my_username", "password").unwrap();

    assert_eq!(response.status(), StatusCode::from_u16(204).unwrap());
}

#[test]
fn register_empty_username() {
    let setup = setup();

    let response = register(&setup.base_url, "", "password");

    assert!(matches!(response, Err(Error::StatusCode(400))));
}

#[test]
fn register_empty_password() {
    let setup = setup();

    let response = register(&setup.base_url, "my_username", "");

    assert!(matches!(response, Err(Error::StatusCode(400))));
}

#[test]
fn register_existing_username() {
    let setup = setup();

    let response = register(&setup.base_url, "my_username", "password").unwrap();

    assert_eq!(response.status(), StatusCode::from_u16(204).unwrap());

    let response = register(&setup.base_url, "my_username", "new_password");

    assert!(matches!(response, Err(Error::StatusCode(409))));
}

#[test]
fn login_successful() {
    let setup = setup();

    let response = register(&setup.base_url, "my_username", "password").unwrap();

    assert_eq!(response.status(), StatusCode::from_u16(204).unwrap());

    let response = login(&setup.base_url, "my_username", "password").unwrap();

    assert_eq!(response.status(), StatusCode::from_u16(200).unwrap());
}

#[test]
fn login_nonexistent_user() {
    let setup = setup();

    let response = login(&setup.base_url, "my_username", "password");

    assert!(matches!(response, Err(Error::StatusCode(401))));
}

#[test]
fn login_invalid_credentials() {
    let setup = setup();

    let response = register(&setup.base_url, "my_username", "password").unwrap();

    assert_eq!(response.status(), StatusCode::from_u16(204).unwrap());

    let response = login(&setup.base_url, "my_username", "DIFFERENT PASSWORD");

    assert!(matches!(response, Err(Error::StatusCode(401))));
}
