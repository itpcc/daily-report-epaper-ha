use serde::Deserialize;
use std::{
    net::{Ipv6Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

pub type Config = Arc<Configuration>;

#[derive(Deserialize)]
pub struct Configuration {
    /// The environment in which to run the application.
    pub env: Environment,

    /// The address to listen on.
    pub listen_address: SocketAddr,
    /// The port to listen on.
    pub app_port: u16,

    /// The DSN for the database. Currently, only PostgreSQL is supported.
    pub db_dsn: String,
    /// The maximum number of connections for the PostgreSQL pool.
    pub db_pool_max_size: u32,

    pub tz: String,

    // * iCal list
    pub ical_holiday: String,
    pub ical_event: String,

    // * Home Assistant
    pub ha_url: String,
    pub ha_token: String,

    // * Authentication
    pub access_token: String,
}

#[derive(Deserialize, Debug)]
pub enum Environment {
    Development,
    Production,
}

impl Configuration {
    /// Creates a new configuration from environment variables.
    pub fn new() -> Config {
        let env = env_var("APP_ENVIRONMENT")
            .parse::<Environment>()
            .expect("Unable to parse the value of the APP_ENVIRONMENT environment variable. Please make sure it is either \"development\" or \"production\".");

        let app_port = env_var("PORT")
            .parse::<u16>()
            .expect("Unable to parse the value of the PORT environment variable. Please make sure it is a valid unsigned 16-bit integer");

        let db_dsn = env_var("DATABASE_URL");

        let db_pool_max_size = env_var("DATABASE_POOL_MAX_SIZE")
            .parse::<u32>()
            .expect("Unable to parse the value of the DATABASE_POOL_MAX_SIZE environment variable. Please make sure it is a valid unsigned 32-bit integer.");

        let listen_address = SocketAddr::from((Ipv6Addr::UNSPECIFIED, app_port));

        let tz = env_var("TZ");

        let ical_holiday = env_var("ICAL_HOLIDAY");
        let ical_event = env_var("ICAL_EVENT");

        let ha_url = env_var("HA_URL");
        let ha_token = env_var("HA_TOKEN");

        let access_token = env_var("ACCESS_TOKEN");

        Arc::new(Configuration {
            env,
            listen_address,
            app_port,
            db_dsn,
            db_pool_max_size,
            tz,
            ical_holiday,
            ical_event,
            ha_url,
            ha_token,
            access_token,
        })
    }

    /// Sets the database DSN.
    /// This method is used in tests to override the database DSN.
    pub fn set_dsn(&mut self, db_dsn: String) {
        self.db_dsn = db_dsn
    }
}

impl FromStr for Environment {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "development" => Ok(Environment::Development),
            "production" => Ok(Environment::Production),
            _ => Err(format!(
                "Invalid environment: {}. Please make sure it is either \"development\" or \"production\".",
                s
            )),
        }
    }
}

pub fn env_var(name: &str) -> String {
    std::env::var(name)
        .map_err(|e| format!("{}: {}", name, e))
        .expect("Missing environment variable")
}
