use std::sync::Arc;

use crate::auth::{AuthSession, LoginCredentials, LoginPayload};
use anyhow::Context;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use password_auth::generate_hash;
use secrecy::ExposeSecret;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app::ApiContext,
    domain::{email_address::EmailAddress, password::Password, username::Username},
};

pub fn router() -> Router<Arc<ApiContext>> {
    Router::new()
        .route("/api/user/register", post(register_user))
        .route("/api/user/login", post(login_user))
        .route("/api/user", get(get_current_user).put(update_user))
}

#[derive(serde::Deserialize)]
struct RegisterPayload {
    email: String,
    username: String,
    password: String,
}

struct RegisterCredentials {
    email: EmailAddress,
    username: Username,
    password: Password,
}

#[derive(serde::Serialize)]
struct UserInfo {
    user_id: Uuid,
    email: String,
    username: String,
}

#[derive(thiserror::Error, Debug)]
enum RegisterError {
    #[error(transparent)]
    InvalidEmail(#[from] crate::domain::email_address::InvalidEmailError),
    #[error(transparent)]
    InvalidUsername(#[from] crate::domain::username::InvalidUsernameError),
    #[error(transparent)]
    InvalidPassword(#[from] crate::domain::password::InvalidPasswordError),
    #[error("Username already exists")]
    UsernameExists,
    #[error("Email already exists")]
    EmailExists,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl TryFrom<RegisterPayload> for RegisterCredentials {
    type Error = RegisterError;

    fn try_from(payload: RegisterPayload) -> Result<Self, Self::Error> {
        let email = EmailAddress::parse(&payload.email)?;
        let username = Username::parse(&payload.username)?;
        let password = Password::parse(&payload.password)?;

        Ok(Self {
            email,
            username,
            password,
        })
    }
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RegisterError::InvalidEmail(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            RegisterError::InvalidUsername(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            RegisterError::InvalidPassword(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            RegisterError::UsernameExists => {
                (StatusCode::CONFLICT, "Username already taken".to_string())
            }
            RegisterError::EmailExists => {
                (StatusCode::CONFLICT, "Email already registered".to_string())
            }
            RegisterError::UnexpectedError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred".to_string(),
            ),
        };
        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

async fn register_user(
    State(api_context): State<Arc<ApiContext>>,
    Json(payload): Json<RegisterPayload>,
) -> Result<(StatusCode, Json<UserInfo>), RegisterError> {
    let register_credentials: RegisterCredentials = payload.try_into()?;

    let mut transaction = api_context
        .db
        .begin()
        .await
        .context("Failed to begin SQL transaction to register a new user")?;

    let user_info = store_register_credentials(&mut transaction, register_credentials).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to register a new user")?;

    Ok((StatusCode::CREATED, Json(user_info)))
}

async fn store_register_credentials(
    transaction: &mut Transaction<'_, Postgres>,
    register_credentials: RegisterCredentials,
) -> Result<UserInfo, RegisterError> {
    let username = register_credentials.username.as_ref();
    let username_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM user_info WHERE username = $1)
        "#,
        username
    )
    .fetch_one(&mut **transaction)
    .await
    .context("Failed to perform query to retrieve username")?;
    if username_exists == Some(true) {
        return Err(RegisterError::UsernameExists);
    }

    let email = register_credentials.email.as_ref();
    let email_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM user_info WHERE email = $1)
        "#,
        email
    )
    .fetch_one(&mut **transaction)
    .await
    .context("Failed to perform query to retrieve email")?;
    if email_exists == Some(true) {
        return Err(RegisterError::EmailExists);
    }

    let user_id = Uuid::new_v4();
    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO user_info (user_id, username, email) VALUES ($1, $2, $3)
            "#,
            user_id,
            username,
            email
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

    Ok(UserInfo {
        user_id,
        email: email.to_owned(),
        username: username.to_owned(),
    })
}

async fn login_user(
    mut auth_session: AuthSession,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    let credentials: LoginCredentials = match payload.try_into() {
        Ok(credentials) => credentials,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::UNAUTHORIZED,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

async fn get_current_user() {
    todo!();
}

async fn update_user() {
    todo!();
}
