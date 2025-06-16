use clap_derive::ValueEnum;
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
    /// Application environment
    #[clap(long, env)]
    pub app_env: AppEnv,
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
    #[clap(long, env)]
    pub database_url: SecretString,
    #[clap(long, env)]
    pub redis_url: SecretString,
}

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq)]
pub enum AppEnv {
    #[clap(name = "development")]
    Development,
    #[clap(name = "staging")]
    Staging,
    #[clap(name = "production")]
    Production,
}
