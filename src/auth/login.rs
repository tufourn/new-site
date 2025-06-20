use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::AppendHeaders;
use axum::{Form, response::IntoResponse};

use crate::auth::{AuthError, AuthSession, LoginCredentials};
use crate::domain::password::Password;
use crate::domain::username::Username;

#[derive(Template, WebTemplate)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {}

pub async fn login_page() -> LoginTemplate {
    LoginTemplate {}
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, error) = match self {
            AuthError::UnexpectedError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occured",
            ),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
        };

        (status_code, error).into_response()
    }
}

#[derive(serde::Deserialize)]
pub struct LoginFormData {
    username: String,
    password: String,
}

impl TryInto<LoginCredentials> for LoginFormData {
    type Error = AuthError;

    fn try_into(self) -> Result<LoginCredentials, Self::Error> {
        let username =
            Username::parse(&self.username).map_err(|_| AuthError::InvalidCredentials)?;
        let password =
            Password::parse(&self.password).map_err(|_| AuthError::InvalidCredentials)?;

        Ok(LoginCredentials { username, password })
    }
}

pub async fn login_user(
    mut auth_session: AuthSession,
    Form(payload): Form<LoginFormData>,
) -> Result<impl IntoResponse, AuthError> {
    let credentials: LoginCredentials = payload.try_into()?;

    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => return Err(AuthError::InvalidCredentials),
        Err(_) => {
            return Err(AuthError::UnexpectedError(anyhow::anyhow!(
                "An internal server error occured"
            )));
        }
    };

    if auth_session.login(&user).await.is_err() {
        return Err(AuthError::UnexpectedError(anyhow::anyhow!(
            "An internal server error occured"
        )));
    }

    Ok((StatusCode::OK, AppendHeaders([("HX-Redirect", "/")])))
}
