use std::{collections::HashMap, io::Read};

use galactic_exchange::{
    AuthToken, Deserializable, Extractable, FieldName, FieldValue, GalacticExchange, Message,
    PasswordRaw, Username, serialize_message,
};
use rouille::{Request, Response};

enum RegistrationError {
    InvalidInput,
    UsernameAlreadyExists,
    PasswordDoesNotMeetConstraints,
}

fn post_register(
    request: &Request,
    exchange: &mut GalacticExchange,
) -> Result<(), RegistrationError> {
    let Some("application/x-galacticbuf") = request.header("Accept") else {
        return Err(RegistrationError::InvalidInput);
    };
    let mut data = request.data().expect("Body already retrieved");
    let mut buf = vec![];
    data.read_to_end(&mut buf)
        .map_err(|_| RegistrationError::InvalidInput)?;

    let (mut message, _) =
        Message::deserialize(&buf, None).map_err(|_| RegistrationError::InvalidInput)?;

    if message.header.field_count != 2 {
        return Err(RegistrationError::InvalidInput);
    }

    let username: Username = message
        .body
        .get_value("username")
        .ok_or(RegistrationError::InvalidInput)?;

    let password: PasswordRaw = message
        .body
        .get_value("password")
        .ok_or(RegistrationError::InvalidInput)?;

    exchange.register(username, password).map_err(|e| match e {
        galactic_exchange::RegistrationError::UsernameAlreadyExists => {
            RegistrationError::UsernameAlreadyExists
        }
        galactic_exchange::RegistrationError::PasswordDoesNotMeetConstraints => {
            RegistrationError::PasswordDoesNotMeetConstraints
        }
    })
}

pub fn post_register_response(request: &Request, exchange: &mut GalacticExchange) -> Response {
    match post_register(request, exchange) {
        Ok(()) => Response::empty_204(),
        Err(RegistrationError::InvalidInput) => Response::empty_400(),
        Err(RegistrationError::UsernameAlreadyExists) => {
            Response::empty_400().with_status_code(409)
        }
        Err(RegistrationError::PasswordDoesNotMeetConstraints) => Response::empty_400(),
    }
}

enum LoginError {
    InvalidInput,
    InvalidCredentials,
    UserDoesNotExist,
}

fn post_login(request: &Request, exchange: &mut GalacticExchange) -> Result<AuthToken, LoginError> {
    let Some("application/x-galacticbuf") = request.header("Accept") else {
        return Err(LoginError::InvalidInput);
    };
    let mut data = request.data().expect("Body already retrieved");
    let mut buf = vec![];
    data.read_to_end(&mut buf)
        .map_err(|_| LoginError::InvalidInput)?;

    let (mut message, _) =
        Message::deserialize(&buf, None).map_err(|_| LoginError::InvalidInput)?;

    if message.header.field_count != 2 {
        return Err(LoginError::InvalidInput);
    }

    let username: Username = message
        .body
        .get_value("username")
        .ok_or(LoginError::InvalidInput)?;

    let password: PasswordRaw = message
        .body
        .get_value("password")
        .ok_or(LoginError::InvalidInput)?;

    exchange.login(username, password).map_err(|e| match e {
        galactic_exchange::AuthenticationError::InvalidCredentials => {
            LoginError::InvalidCredentials
        }
        galactic_exchange::AuthenticationError::UsernameDoesNotExist => {
            LoginError::UserDoesNotExist
        }
        galactic_exchange::AuthenticationError::InvalidAuthToken => LoginError::InvalidCredentials,
    })
}

pub fn post_login_response(request: &Request, exchange: &mut GalacticExchange) -> Response {
    match post_login(request, exchange) {
        Ok(auth_token) => {
            let body: HashMap<FieldName, FieldValue> = [("token".into(), auth_token.into())].into();
            match serialize_message(body) {
                Ok(data) => Response::from_data("application/x-galacticbuf", data),
                Err(_) => Response::empty_400().with_status_code(500),
            }
        }
        Err(LoginError::InvalidInput) => Response::empty_400(),
        Err(LoginError::UserDoesNotExist) => Response::empty_400().with_status_code(401),
        Err(LoginError::InvalidCredentials) => Response::empty_400().with_status_code(401),
    }
}
