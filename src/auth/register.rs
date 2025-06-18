use std::sync::Arc;

use anyhow::Context;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{AppendHeaders, IntoResponse},
};
use password_auth::generate_hash;
use secrecy::ExposeSecret;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app::ApiContext,
    domain::{
        email_address::{EmailAddress, InvalidEmailError},
        password::{InvalidPasswordError, Password},
        username::{InvalidUsernameError, Username},
    },
};

#[derive(Template, WebTemplate)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {}

pub async fn get() -> RegisterTemplate {
    RegisterTemplate {}
}

#[derive(serde::Deserialize)]
pub struct RegisterFormData {
    email: String,
    username: String,
    password: String,
}

struct RegisterCredentials {
    pub email: EmailAddress,
    pub username: Username,
    pub password: Password,
}

#[derive(thiserror::Error, Debug)]
pub enum RegisterError {
    #[error("Invalid email address")]
    InvalidEmail(#[from] InvalidEmailError),
    #[error("Invalid username")]
    InvalidUsername(#[from] InvalidUsernameError),
    #[error("Invalid password")]
    InvalidPassword(#[from] InvalidPasswordError),
    #[error("Email already exists")]
    EmailExists,
    #[error("Username already exists")]
    UsernameExists,
    #[error("An internal server error occured")]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            RegisterError::InvalidEmail(_)
            | RegisterError::InvalidUsername(_)
            | RegisterError::InvalidPassword(_) => StatusCode::BAD_REQUEST,
            RegisterError::UsernameExists | RegisterError::EmailExists => StatusCode::CONFLICT,
            RegisterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status_code, self.to_string()).into_response()
    }
}

pub async fn register_user(
    State(api_context): State<Arc<ApiContext>>,
    Form(form_data): Form<RegisterFormData>,
) -> Result<impl IntoResponse, RegisterError> {
    let email = validate_email(&form_data.email, &api_context.db).await?;
    let username = validate_username(&form_data.username, &api_context.db).await?;
    let password = Password::parse(&form_data.password)?;

    let mut transaction = api_context
        .db
        .begin()
        .await
        .context("Failed to begin transaction")?;

    let register_credentials = RegisterCredentials {
        email,
        username,
        password,
    };

    store_register_credentials(&mut transaction, register_credentials).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok((
        StatusCode::CREATED,
        AppendHeaders([("HX-Redirect", "/login")]),
    ))
}

async fn validate_username(username_str: &str, db: &PgPool) -> Result<Username, RegisterError> {
    let username = Username::parse(username_str)?;
    let username_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM user_info WHERE username = $1)
        "#,
        username.as_ref()
    )
    .fetch_one(db)
    .await
    .context("Failed to perform query to retrieve username")?;
    if username_exists == Some(true) {
        Err(RegisterError::UsernameExists)
    } else {
        Ok(username)
    }
}

async fn validate_email(email_str: &str, db: &PgPool) -> Result<EmailAddress, RegisterError> {
    let email = EmailAddress::parse(email_str)?;
    let email_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM user_info WHERE email = $1)
        "#,
        email.as_ref()
    )
    .fetch_one(db)
    .await
    .context("Failed to perform query to retrieve email")?;
    if email_exists == Some(true) {
        Err(RegisterError::EmailExists)
    } else {
        Ok(email)
    }
}

async fn store_register_credentials(
    transaction: &mut Transaction<'_, Postgres>,
    register_credentials: RegisterCredentials,
) -> Result<(), anyhow::Error> {
    let user_id = Uuid::new_v4();
    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO user_info (user_id, username, email) VALUES ($1, $2, $3)
            "#,
            user_id,
            register_credentials.username.as_ref(),
            register_credentials.email.as_ref()
        ))
        .await
        .context("Failed to insert user info into user_info table")?;

    let password_hash = generate_hash(register_credentials.password.expose_secret().as_bytes());
    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO user_password (user_id, password_hash) VALUES ($1, $2)
            "#,
            user_id,
            password_hash,
        ))
        .await
        .context("Failed to insert user password into user_password table")?;

    Ok(())
}
