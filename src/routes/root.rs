use askama::Template;
use askama_web::WebTemplate;
use axum::response::IntoResponse;

#[derive(Template, WebTemplate)]
#[template(path = "root.html")]
struct RootTemplate;

pub async fn get_homepage() -> impl IntoResponse {
    RootTemplate.into_response()
}
