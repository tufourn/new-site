use anyhow::Context;
use async_trait::async_trait;
use axum::{
    Router,
    routing::{get, post},
};
use axum_login::{AuthUser, AuthnBackend, UserId};
use password_auth::verify_password;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, prelude::FromRow};
use uuid::Uuid;

use crate::{
    app::AppRouter,
    domain::{password::Password, username::Username},
};

mod login;
mod logout;
mod register;

pub fn router() -> AppRouter {
    Router::new()
        .route("/register", get(register::register_page))
        .route("/login", get(login::login_page))
        .route("/logout", get(logout::logout))
        .route("/api/register", post(register::register_user))
        .route("/api/login", post(login::login_user))
}

#[derive(Clone, Debug, FromRow)]
pub struct User {
    user_id: Uuid,
    pub username: String,
    password_hash: SecretString,
}

impl User {
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.expose_secret().as_bytes()
    }
}

pub struct LoginCredentials {
    username: Username,
    password: Password,
}

#[derive(Clone, Debug)]
pub struct Backend {
    db: PgPool,
}

impl Backend {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Invalid credentials")]
    InvalidCredentials,
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = LoginCredentials;
    type Error = AuthError;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<Self::User> = sqlx::query_as!(
            Self::User,
            r#"
            SELECT ui.user_id, ui.username, up.password_hash
            FROM user_info AS ui JOIN user_password AS up
                ON ui.user_id = up.user_id
            WHERE ui.username = $1
            "#,
            credentials.username.as_ref(),
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch stored user credentials")?;

        tokio::task::spawn_blocking(move || {
            Ok(user.filter(|user| {
                verify_password(
                    credentials.password.expose_secret().as_bytes(),
                    user.password_hash.expose_secret(),
                )
                .is_ok()
            }))
        })
        .await
        .context("Failed to spawn blocking task")?
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<Self::User> = sqlx::query_as!(
            Self::User,
            r#"
            SELECT ui.user_id, ui.username, up.password_hash
            FROM user_info AS ui JOIN user_password AS up
                ON ui.user_id = up.user_id
            WHERE ui.user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to get user")?;

        Ok(user)
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
