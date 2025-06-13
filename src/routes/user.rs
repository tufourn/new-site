use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    app::ApiContext,
    domain::{
        email_address::{EmailAddress, InvalidEmailError},
        password::{InvalidPasswordError, Password},
        username::{InvalidUsernameError, Username},
    },
};

pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/api/user/register", post(register_user))
        .route("/api/user/login", post(login_user))
        .route("/api/user", get(get_current_user).put(update_user))
}

struct RegisterCredentials {
    email: EmailAddress,
    username: Username,
    password: Password,
}

#[derive(thiserror::Error, Debug)]
enum RegisterError {
    #[error("Invalid email")]
    InvalidEmail(#[source] InvalidEmailError),
    #[error("Invalid username")]
    InvalidUsername(#[source] InvalidUsernameError),
    #[error("Invalid password")]
    InvalidPassword(#[source] InvalidPasswordError),
}

async fn register_user() {
    todo!();
}

async fn login_user() {
    todo!();
}

async fn get_current_user() {
    todo!();
}

async fn update_user() {
    todo!();
}
