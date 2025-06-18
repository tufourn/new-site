use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::AppendHeaders;
use axum::{Form, response::IntoResponse};

use crate::auth::{AuthError, AuthSession, LoginCredentials, LoginFormData};

#[derive(Template, WebTemplate)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {}

pub async fn get() -> LoginTemplate {
    LoginTemplate {}
}

// protected page for testing, remove later
#[derive(Template, WebTemplate)]
#[template(path = "auth/protected.html")]
pub struct ProtectedTemplate {}

pub async fn get_protected(auth_session: AuthSession) -> impl IntoResponse {
    let protected = ProtectedTemplate {};
    match auth_session.user {
        Some(_user) => protected.into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
//

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

pub async fn post(
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

    Ok((
        StatusCode::OK,
        AppendHeaders([("HX-Redirect", "/protected")]),
    ))
}
