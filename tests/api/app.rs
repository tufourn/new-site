use std::str::FromStr;

use clap::Parser;
use secrecy::SecretString;
use site::{app::Application, config::Config};
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

    config.application_settings.app_port = 0;

    let db_url_without_db = "postgresql://postgres:password@localhost:5432";
    let db_name = Uuid::new_v4().to_string();
    config.database_settings.database_url =
        SecretString::from(format!("{}/{}", db_url_without_db, db_name));

    let mut connection = PgConnection::connect(db_url_without_db)
        .await
        .expect("Failed to initialize Postgres connection");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
        .await
        .expect("Failed to create database");

    let connect_options = PgConnectOptions::from_str(db_url_without_db)
        .expect("Failed to parse database url")
        .ssl_mode(PgSslMode::Prefer)
        .database(&db_name);

    let db = PgPool::connect_with(connect_options)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to migrate the database");

    let app = Application::build(config).await;

    let address = format!("http://localhost:{}", app.port());

    let client = reqwest::Client::builder().build().unwrap();

    let _task = tokio::spawn(async move { app.run().await });

    TestApp {
        address,
        db,
        client,
    }
}
