use std::{
    collections::HashMap,
    fmt::Display,
    time::{Duration, Instant},
};

use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use rand::{RngExt, distr::Alphanumeric};

use crate::app::galacticbuf::{FieldValue, StringValue};

pub enum RegistrationError {
    UsernameAlreadyExists,
    PasswordDoesNotMeetConstraints,
}

#[derive(Debug)]
pub enum AuthenticationError {
    UsernameDoesNotExist,
    InvalidCredentials,
    InvalidAuthToken,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UserId(u64);

struct Sessions {
    sessions: HashMap<AuthToken, Session>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Username(String);

#[derive(PartialEq, Eq, Clone)]
pub struct PasswordRaw(String);

#[derive(Debug)]
struct PasswordHash(String);

#[derive(Debug)]
struct User {
    id: UserId,
    username: Username,
    password: PasswordHash,
}

pub struct Auth {
    ids: Ids,
    users: HashMap<UserId, User>,
    sessions: Sessions,
}

struct Ids {
    next: UserId,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct AuthToken(String);

struct Session {
    valid_until: Instant,
    user_id: UserId,
}

pub enum UsernameError {
    Invalid,
    EmptyUsername,
}

impl From<AuthToken> for FieldValue {
    fn from(value: AuthToken) -> Self {
        FieldValue::String(StringValue(value.0))
    }
}

impl TryFrom<FieldValue> for Username {
    type Error = UsernameError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into().map_err(|_| UsernameError::Invalid)?;

        if value.is_empty() {
            return Err(UsernameError::EmptyUsername);
        }

        Ok(Self(value))
    }
}

impl From<&str> for Username {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub enum PasswordRawError {
    Invalid,
    EmptyPassword,
}

impl TryFrom<FieldValue> for PasswordRaw {
    type Error = PasswordRawError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into().map_err(|_| PasswordRawError::Invalid)?;

        if value.is_empty() {
            return Err(PasswordRawError::EmptyPassword);
        }

        Ok(Self(value))
    }
}

impl From<&str> for PasswordRaw {
    fn from(value: &str) -> Self {
        PasswordRaw(String::from(value))
    }
}

fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[derive(Debug)]
struct HashError;

fn hash_password(password: &str) -> Result<PasswordHash, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let valid_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            dbg!(e);
            HashError
        })?
        .to_string();

    Ok(PasswordHash(valid_hash))
}

fn verify_password(password: &str, password_hash: &PasswordHash) -> bool {
    let parsed_hash = argon2::PasswordHash::new(&password_hash.0)
        .expect("Stored hash should always be a valid hash");
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

impl PasswordRaw {
    pub fn new(password: String) -> Option<Self> {
        if password.is_empty() {
            None
        } else {
            Some(PasswordRaw(password))
        }
    }
}

impl Sessions {
    const SESSION_LENGTH: Duration = Duration::from_hours(1);
    const TOKEN_LENGTH: usize = 25;

    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    fn authenticate(&self, token: AuthToken) -> Result<UserId, AuthenticationError> {
        let session = self
            .sessions
            .get(&token)
            .ok_or(AuthenticationError::InvalidAuthToken)?;

        if Instant::now() < session.valid_until {
            Ok(session.user_id)
        } else {
            Err(AuthenticationError::InvalidAuthToken)
        }
    }

    fn new_session(&mut self, user_id: UserId) -> AuthToken {
        self.sessions.retain(|_, s| s.user_id != user_id);
        let token = AuthToken(random_string(Self::TOKEN_LENGTH));
        self.sessions.insert(
            token.clone(),
            Session {
                valid_until: Instant::now() + Self::SESSION_LENGTH,
                user_id,
            },
        );
        token
    }
}

impl Ids {
    fn new() -> Self {
        Self { next: UserId(0) }
    }

    fn next_id(&mut self) -> UserId {
        let id = self.next;
        self.next = UserId(self.next.0 + 1);
        id
    }
}

impl Auth {
    pub fn new() -> Self {
        Self {
            ids: Ids::new(),
            users: HashMap::new(),
            sessions: Sessions::new(),
        }
    }

    pub fn register(
        &mut self,
        username: Username,
        password: PasswordRaw,
    ) -> Result<(), RegistrationError> {
        if self.users.values().any(|user| user.username == username) {
            return Err(RegistrationError::UsernameAlreadyExists);
        }

        let id = self.ids.next_id();

        let password_hash = hash_password(&password.0)
            .map_err(|_| RegistrationError::PasswordDoesNotMeetConstraints)?;

        match self.users.insert(
            id,
            User {
                id,
                username,
                password: password_hash,
            },
        ) {
            Some(_) => panic!("User with the same id already exists"),
            None => {
                println!("Registered user with id {:?}", id);
                Ok(())
            }
        }
    }

    pub fn login(
        &mut self,
        username: Username,
        password: PasswordRaw,
    ) -> Result<AuthToken, AuthenticationError> {
        let user = self
            .users
            .values()
            .find(|u| u.username == username)
            .ok_or(AuthenticationError::UsernameDoesNotExist)?;

        if !verify_password(&password.0, &user.password) {
            return Err(AuthenticationError::InvalidCredentials);
        }

        Ok(self.sessions.new_session(user.id))
    }

    pub fn authenticate(&self, token: AuthToken) -> Result<UserId, AuthenticationError> {
        self.sessions.authenticate(token)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next_id() {
        let mut ids1 = Ids::new();
        let id1 = ids1.next_id();
        let id2 = ids1.next_id();

        assert_ne!(id1, id2);
    }

    #[test]
    fn register_repeat_username_fails() {
        let mut auth = Auth::new();
        let u1: Username = "u1".into();
        let p1: PasswordRaw = "p1".into();
        let res = auth.register(u1.clone(), p1);
        assert!(res.is_ok());

        let p2: PasswordRaw = "p2".into();
        let res = auth.register(u1, p2);

        assert!(matches!(res, Err(RegistrationError::UsernameAlreadyExists)));
    }

    #[test]
    fn successful_login() {
        let mut auth = Auth::new();
        let u1: Username = "u1".into();
        let p1: PasswordRaw = "p1".into();

        let res = auth.register(u1.clone(), p1.clone());
        assert!(matches!(res, Ok(())));

        let res = auth.login(u1, p1);
        dbg!(&res);
        assert!(res.is_ok());
    }

    #[test]
    fn password_hashing() {
        let p = "password";
        let hashed = hash_password(p).unwrap();
        assert!(verify_password(p, &hashed));
    }
}
