use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use secrecy::ExposeSecret;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app::ApiContext,
    domain::{email_address::EmailAddress, password::Password, username::Username},
};

pub fn router() -> Router<ApiContext> {
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
    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("Password hashing error")]
    PasswordHashingError(#[from] argon2::password_hash::Error),
}

impl TryFrom<RegisterPayload> for RegisterCredentials {
    type Error = RegisterError;

    fn try_from(payload: RegisterPayload) -> Result<Self, Self::Error> {
        let email = EmailAddress::parse(payload.email)?;
        let username = Username::parse(payload.username)?;
        let password = Password::parse(payload.password)?;

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
            RegisterError::DbError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error occurred".to_string(),
            ),
            RegisterError::PasswordHashingError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to securely process password".to_string(),
            ),
        };
        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

async fn register_user(
    State(api_context): State<ApiContext>,
    Json(payload): Json<RegisterPayload>,
) -> Result<StatusCode, RegisterError> {
    let register_credentials: RegisterCredentials = payload.try_into()?;

    let mut transaction = api_context.db.begin().await?;

    store_register_credentials(&mut transaction, register_credentials).await?;

    transaction.commit().await?;

    Ok(StatusCode::CREATED)
}

async fn store_register_credentials(
    transaction: &mut Transaction<'_, Postgres>,
    register_credentials: RegisterCredentials,
) -> Result<(), RegisterError> {
    let username = register_credentials.username.as_ref();
    let username_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM user_info WHERE username = $1)
        "#,
        username
    )
    .fetch_one(&mut **transaction)
    .await?;
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
    .await?;
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
        .map_err(RegisterError::DbError)?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(
            register_credentials.password.expose_secret().as_bytes(),
            &salt,
        )?
        .to_string();
    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO user_password (user_id, password_hash) VALUES ($1, $2)
            "#,
            user_id,
            password_hash,
        ))
        .await
        .map_err(RegisterError::DbError)?;

    Ok(())
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
