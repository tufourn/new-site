use axum::response::{AppendHeaders, IntoResponse};
use http::StatusCode;

use crate::auth::AuthSession;

pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => (StatusCode::OK, AppendHeaders([("HX-Redirect", "/login")])).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
