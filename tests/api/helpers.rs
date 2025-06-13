use clap::Parser;
use secrecy::ExposeSecret;
use site::{
    app::Application,
    config::{Config, DatabaseSettings},
};
use sqlx::{
    Connection, Executor, PgConnection, PgPool,
    postgres::{PgConnectOptions, PgSslMode},
};
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db: PgPool,
    pub client: reqwest::Client,
}

pub async fn spawn_app() -> TestApp {
    dotenvy::dotenv().ok();
    let mut config = Config::parse();

    config.database_settings.database_name = Uuid::new_v4().to_string();
    config.application_settings.app_port = 0;

    let db = configure_database(&config.database_settings).await;
    let app = Application::build(config).await;

    let address = format!("http://localhost:{}", app.port());

    let client = reqwest::Client::builder().build().unwrap();

    let _ = tokio::spawn(async move { app.run().await });

    let test_app = TestApp {
        address,
        db,
        client,
    };

    test_app
}

pub async fn configure_database(db_settings: &DatabaseSettings) -> PgPool {
    let ssl_mode = if db_settings.database_sslmode {
        PgSslMode::Require
    } else {
        PgSslMode::Prefer
    };
    let connect_options = PgConnectOptions::new()
        .host(&db_settings.database_host)
        .port(db_settings.database_port)
        .username(&db_settings.database_username)
        .password(&db_settings.database_password.expose_secret())
        .ssl_mode(ssl_mode);

    let mut connection = PgConnection::connect_with(&connect_options)
        .await
        .expect("Failed to initialize Postgres connection");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connect_options = connect_options.database(&db_settings.database_name);
    let connection_pool = PgPool::connect_with(connect_options)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
