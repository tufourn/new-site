use crate::app::{TestApp, spawn_app};

async fn register_user(app: &TestApp, body: serde_json::Value) -> reqwest::Response {
    app.client
        .post(format!("{}/api/user/register", app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request")
}

async fn login_user(app: &TestApp, body: serde_json::Value) -> reqwest::Response {
    app.client
        .post(format!("{}/api/user/login", app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request")
}

#[tokio::test]
async fn register_with_valid_credentials_returns_201() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": "test@example.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });

    let response = register_user(&app, body).await;
    assert_eq!(201, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, username FROM user_info")
        .fetch_one(&app.db)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "test@example.com");
    assert_eq!(saved.username, "testuser");

    let saved = sqlx::query!("SELECT password_hash FROM user_password")
        .fetch_one(&app.db)
        .await
        .expect("Failed to fetch saved subscription");
    assert_ne!(saved.password_hash, "correct horse battery staple");
}

#[tokio::test]
async fn register_with_invalid_credentials_returns_400() {
    let app = spawn_app().await;

    let body_invalid_email = serde_json::json!({
        "email": "test.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, body_invalid_email).await;
    assert_eq!(400, response.status().as_u16());

    let body_invalid_username = serde_json::json!({
        "email": "test@test.com",
        "username": "@",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, body_invalid_username).await;
    assert_eq!(400, response.status().as_u16());

    let body_invalid_password = serde_json::json!({
        "email": "test@test.com",
        "username": "testuser",
        "password": "hunter2",
    });
    let response = register_user(&app, body_invalid_password).await;
    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn register_with_missing_fields_returns_422() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": "test@test.com",
    });
    let response = register_user(&app, body).await;
    assert_eq!(422, response.status().as_u16());
}

#[tokio::test]
async fn duplicate_user_name_or_email_returns_409() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": "test@test.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, body).await;
    assert_eq!(201, response.status().as_u16());

    let body_duplicate_email = serde_json::json!({
        "email": "test@test.com",
        "username": "testuser_different",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, body_duplicate_email).await;
    assert_eq!(409, response.status().as_u16());

    let body_duplicate_username = serde_json::json!({
        "email": "test_different@test.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, body_duplicate_username).await;
    assert_eq!(409, response.status().as_u16());
}

#[tokio::test]
async fn login_with_valid_credentials_returns_200() {
    let app = spawn_app().await;

    let register_body = serde_json::json!({
        "email": "test@test.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, register_body).await;
    assert_eq!(201, response.status().as_u16());

    let login_body = serde_json::json!({
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = login_user(&app, login_body).await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn login_with_invalid_credentials_returns_401() {
    let app = spawn_app().await;

    let register_body = serde_json::json!({
        "email": "test@test.com",
        "username": "testuser",
        "password": "correct horse battery staple",
    });
    let response = register_user(&app, register_body).await;
    assert_eq!(201, response.status().as_u16());

    let login_body = serde_json::json!({
        "username": "testuser",
        "password": "hunter2",
    });
    let response = login_user(&app, login_body).await;
    assert_eq!(401, response.status().as_u16());
}
