pub mod auth;
pub mod galacticbuf;

use crate::app::auth::{
    Auth, AuthToken, AuthenticationError, PasswordRaw, RegistrationError, UserId, Username,
};

pub struct GalacticExchange {
    auth: Auth,
}

impl GalacticExchange {
    pub fn new() -> Self {
        Self { auth: Auth::new() }
    }

    pub fn register(
        &mut self,
        username: Username,
        password: PasswordRaw,
    ) -> Result<(), RegistrationError> {
        println!("[INFO] Register: ({username}, ...)");
        self.auth.register(username, password)
    }

    pub fn login(
        &mut self,
        username: Username,
        password: PasswordRaw,
    ) -> Result<AuthToken, AuthenticationError> {
        println!("[INFO] Login: ({username}, ...)");
        self.auth.login(username, password)
    }

    pub fn authenticate(&self, token: AuthToken) -> Result<UserId, AuthenticationError> {
        self.auth.authenticate(token)
    }
}
