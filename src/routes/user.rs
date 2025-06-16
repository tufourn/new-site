use std::sync::Arc;

use anyhow::Context;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::{Executor, PgPool, Postgres, Transaction};
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

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(
            register_credentials.password.expose_secret().as_bytes(),
            &salt,
        )
        .context("Failed to hash password")?
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
        .context("Failed to insert user password into user_password table")?;

    Ok(UserInfo {
        user_id,
        email: email.to_owned(),
        username: username.to_owned(),
    })
}

#[derive(serde::Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

struct LoginCredentials {
    username: Username,
    password: Password,
}

#[derive(thiserror::Error, Debug)]
enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl TryFrom<LoginPayload> for LoginCredentials {
    type Error = AuthError;

    fn try_from(payload: LoginPayload) -> Result<Self, Self::Error> {
        let username = Username::parse(&payload.username)
            .context("Invalid username")
            .map_err(AuthError::InvalidCredentials)?;
        let password = Password::parse(&payload.password)
            .context("Invalid password")
            .map_err(AuthError::InvalidCredentials)?;

        Ok(Self { username, password })
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidCredentials(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            ),
            AuthError::UnexpectedError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred".to_string(),
            ),
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

async fn login_user(
    State(api_context): State<Arc<ApiContext>>,
    Json(payload): Json<LoginPayload>,
) -> Result<(), AuthError> {
    let login_credentials: LoginCredentials = payload.try_into()?;
    validate_credentials(login_credentials, &api_context.db).await?;

    Ok(())
}

async fn validate_credentials(
    credentials: LoginCredentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno",
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    tokio::task::spawn_blocking(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username"))
        .map_err(AuthError::InvalidCredentials)
}

async fn get_stored_credentials(
    username: Username,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, AuthError> {
    let row = sqlx::query!(
        r#"
        SELECT ui.user_id, up.password_hash
        FROM user_info AS ui JOIN user_password AS up
            ON ui.user_id = up.user_id
        WHERE ui.username = $1
        "#,
        username.as_ref()
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|row| (row.user_id, SecretString::from(row.password_hash)));

    Ok(row)
}

fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: Password,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Wrong password")
        .map_err(AuthError::InvalidCredentials)
}

async fn get_current_user() {
    todo!();
}

async fn update_user() {
    todo!();
}
