use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use password_auth::verify_password;
use secrecy::ExposeSecret;
use sqlx::{PgPool, prelude::FromRow};
use tokio::task;
use uuid::Uuid;

use crate::domain::{password::Password, username::Username};

#[derive(Clone, Debug, FromRow)]
pub struct User {
    user_id: Uuid,
    pub username: String,
    password_hash: String,
}

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
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

#[derive(serde::Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
}

impl TryFrom<LoginPayload> for LoginCredentials {
    type Error = AuthError;

    fn try_from(payload: LoginPayload) -> Result<Self, Self::Error> {
        let username =
            Username::parse(&payload.username).map_err(|_| AuthError::InvalidCredentials)?;
        let password =
            Password::parse(&payload.password).map_err(|_| AuthError::InvalidCredentials)?;

        Ok(Self { username, password })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    TaskJoin(#[from] task::JoinError),

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
        .await?;

        tokio::task::spawn_blocking(move || {
            Ok(user.filter(|user| {
                verify_password(
                    credentials.password.expose_secret().as_bytes(),
                    &user.password_hash,
                )
                .is_ok()
            }))
        })
        .await?
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<Self::User> = sqlx::query_as!(
            Self::User,
            r#"
            SELECT ui.user_id, ui.username, up.password_hash
            FROM user_info AS ui JOIN user_password AS up
                ON ui.user_id = up.user_id
            WHERE ui.username = $1
            "#,
            user_id.to_string()
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user)
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
