use std::sync::Arc;

use axum::Router;
use secrecy::ExposeSecret;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tokio::net::TcpListener;

use crate::{
    config::Config,
    routes::{health_check, user},
};

pub struct Application {
    app: Router,
    listener: TcpListener,
}

pub struct ApiContext {
    pub config: Config,
    pub db: PgPool,
}

impl Application {
    pub async fn build(config: Config) -> Self {
        let ssl_mode = if config.database_settings.database_sslmode {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        let db = PgPoolOptions::new()
            .connect_with(
                PgConnectOptions::new()
                    .host(&config.database_settings.database_host)
                    .port(config.database_settings.database_port)
                    .username(&config.database_settings.database_username)
                    .password(config.database_settings.database_password.expose_secret())
                    .database(&config.database_settings.database_name)
                    .ssl_mode(ssl_mode),
            )
            .await
            .expect("Failed to connect to Postgres");

        sqlx::migrate!()
            .run(&db)
            .await
            .expect("Failed to run migrations");

        let address = format!(
            "{}:{}",
            config.application_settings.app_host, config.application_settings.app_port
        );

        let api_context = ApiContext { config, db };

        let app = api_router().with_state(Arc::new(api_context));

        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to bind port");

        Application { app, listener }
    }

    pub async fn run(self) {
        axum::serve(self.listener, self.app).await.unwrap();
    }

    pub fn address(&self) -> String {
        self.listener.local_addr().unwrap().to_string()
    }

    pub fn port(&self) -> String {
        self.listener.local_addr().unwrap().port().to_string()
    }
}

fn api_router() -> Router<Arc<ApiContext>> {
    health_check::router().merge(user::router())
}
