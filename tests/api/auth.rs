use crate::app::{TestApp, spawn_app};

#[derive(serde::Serialize)]
struct LoginFormData {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
struct RegisterFormData {
    email: String,
    username: String,
    password: String,
}

async fn register_user(app: &TestApp, params: RegisterFormData) -> reqwest::Response {
    app.client
        .post(format!("{}/api/register", app.address))
        .form(&params)
        .send()
        .await
        .expect("Failed to execute request")
}

async fn login_user(app: &TestApp, params: LoginFormData) -> reqwest::Response {
    app.client
        .post(format!("{}/api/login", app.address))
        .form(&params)
        .send()
        .await
        .expect("Failed to execute request")
}

#[tokio::test]
async fn register_with_valid_credentials_returns_201() {
    let app = spawn_app().await;

    let body = RegisterFormData {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };

    let response = register_user(&app, body).await;
    assert_eq!(201, response.status().as_u16());

    let saved_info =
        sqlx::query!("SELECT email, username FROM user_info WHERE email = 'test@example.com'")
            .fetch_one(&app.db)
            .await
            .expect("Failed to fetch saved user info");
    assert_eq!(saved_info.email, "test@example.com");
    assert_eq!(saved_info.username, "testuser");

    let saved_password = sqlx::query!("SELECT password_hash FROM user_password")
        .fetch_one(&app.db)
        .await
        .expect("Failed to fetch saved password hash");
    assert_ne!(saved_password.password_hash, "correct horse battery staple");
}

#[tokio::test]
async fn register_with_invalid_credentials_returns_400() {
    let app = spawn_app().await;

    let body_invalid_email = RegisterFormData {
        email: "test.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, body_invalid_email).await;
    assert_eq!(400, response.status().as_u16());

    let body_invalid_username = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "@".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, body_invalid_username).await;
    assert_eq!(400, response.status().as_u16());

    let body_invalid_password = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "testuser".to_string(),
        password: "hunter2".to_string(),
    };
    let response = register_user(&app, body_invalid_password).await;
    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn duplicate_user_name_or_email_returns_409() {
    let app = spawn_app().await;

    let body = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, body).await;
    assert_eq!(201, response.status().as_u16());

    let body_duplicate_email = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "testuser_different".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, body_duplicate_email).await;
    assert_eq!(409, response.status().as_u16());

    let body_duplicate_username = RegisterFormData {
        email: "test_different@test.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, body_duplicate_username).await;
    assert_eq!(409, response.status().as_u16());
}

#[tokio::test]
async fn login_with_valid_credentials_returns_200() {
    let app = spawn_app().await;

    let register_body = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, register_body).await;
    assert_eq!(201, response.status().as_u16());

    let login_body = LoginFormData {
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = login_user(&app, login_body).await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn login_with_incorrect_credentials_returns_401() {
    let app = spawn_app().await;

    let register_body = RegisterFormData {
        email: "test@test.com".to_string(),
        username: "testuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = register_user(&app, register_body).await;
    assert_eq!(201, response.status().as_u16());

    let login_body = LoginFormData {
        username: "testuser".to_string(),
        password: "hunter2".to_string(),
    };
    let response = login_user(&app, login_body).await;
    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn login_with_unregistered_credentials_returns_401() {
    let app = spawn_app().await;

    let login_body = LoginFormData {
        username: "nonexistentuser".to_string(),
        password: "correct horse battery staple".to_string(),
    };
    let response = login_user(&app, login_body).await;
    assert_eq!(401, response.status().as_u16());
}
