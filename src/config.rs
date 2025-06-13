use secrecy::SecretString;

#[derive(clap::Parser, Debug)]
pub struct Config {
    /// Application settings
    #[clap(flatten)]
    pub application_settings: ApplicationSettings,
    /// The Postgres database url for the application
    #[clap(flatten)]
    pub database_settings: DatabaseSettings,
}

#[derive(clap::Parser, Debug)]
pub struct ApplicationSettings {
    /// Application host
    #[clap(long, env)]
    pub app_host: String,
    /// Application port
    #[clap(long, env)]
    pub app_port: u16,
    /// HMAC key for signing and verification
    #[clap(long, env)]
    pub hmac_key: SecretString,
}

#[derive(clap::Parser, Debug)]
pub struct DatabaseSettings {
    /// Postgres database host
    #[clap(long, env)]
    pub database_host: String,
    /// Postgres database port
    #[clap(long, env)]
    pub database_port: u16,
    /// Postgres database name
    #[clap(long, env)]
    pub database_name: String,
    /// Postgres database username
    #[clap(long, env)]
    pub database_username: String,
    /// Postgres database password
    #[clap(long, env)]
    pub database_password: SecretString,
    /// Postgres ssl mode
    #[clap(long, env, default_value_t = false)]
    pub database_sslmode: bool,
}
