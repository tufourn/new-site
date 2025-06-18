use axum::{Router, routing::get};

use crate::app::AppRouter;

pub fn router() -> AppRouter {
    Router::new().route("/health_check", get(health_check))
}

async fn health_check() {}
