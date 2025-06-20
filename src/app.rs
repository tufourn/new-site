use std::{str::FromStr, sync::Arc};

use axum::{Router, routing::get};
use axum_login::AuthManagerLayerBuilder;
use axum_messages::MessagesManagerLayer;
use fred::{interfaces::ClientLike, prelude::ReconnectPolicy};
use secrecy::ExposeSecret;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_sessions::SessionManagerLayer;
use tower_sessions_redis_store::RedisStore;

use crate::{
    auth,
    config::{self, AppEnv, Config},
    routes::{health_check, root::get_homepage, todo},
};

pub struct Application {
    app: Router,
    listener: TcpListener,
}

pub struct ApiContext {
    pub config: Config,
    pub db: PgPool,
}

pub type AppRouter = Router<Arc<ApiContext>>;

impl Application {
    pub async fn build(config: Config) -> Self {
        let app_env = config.application_settings.app_env;
        let ssl_mode = match app_env {
            config::AppEnv::Development => PgSslMode::Prefer,
            config::AppEnv::Staging | config::AppEnv::Production => PgSslMode::Require,
        };

        let db_connect_options =
            PgConnectOptions::from_str(config.database_settings.database_url.expose_secret())
                .expect("Failed to parse database url")
                .ssl_mode(ssl_mode);

        let db = PgPoolOptions::new()
            .connect_with(db_connect_options)
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

        let redis_config = fred::prelude::Config::from_url(&format!(
            "{}/1",
            &config.database_settings.redis_url.expose_secret()
        ))
        .expect("Failed to configure redis client");

        let redis_pool = fred::prelude::Builder::from_config(redis_config)
            .with_connection_config(|redis_config| {
                redis_config.connection_timeout = std::time::Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(100)
            .expect("Failed to create redis pool");

        redis_pool.init().await.expect("Failed to connect to redis");

        let key = cookie::Key::from(
            config
                .application_settings
                .hmac_key
                .expose_secret()
                .as_bytes(),
        );

        let session_store = RedisStore::new(redis_pool);
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(app_env == AppEnv::Production || app_env == AppEnv::Staging)
            .with_expiry(tower_sessions::Expiry::OnInactivity(
                cookie::time::Duration::seconds(3600),
            ))
            .with_signed(key);

        let backend = crate::auth::Backend::new(db.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let serve_dir = ServeDir::new("assets");

        let api_context = ApiContext { config, db };

        let app = api_router()
            .with_state(Arc::new(api_context))
            .layer(MessagesManagerLayer)
            .layer(auth_layer)
            .nest_service("/assets", serve_dir);

        let listener = TcpListener::bind(address)
            .await
            .expect("Failed to bind port");

        Self { app, listener }
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

fn api_router() -> AppRouter {
    Router::new()
        .route("/", get(get_homepage))
        .merge(health_check::router())
        .merge(todo::router())
        .merge(auth::router())
}
