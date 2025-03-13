use std::sync::Arc;

use server::{Configuration, Db, cron, model::CalendarMap, telemetry};
use time::{OffsetDateTime, PrimitiveDateTime};
use tokio::{net::TcpListener, sync::RwLock};

#[tokio::main]
async fn main() {
    // Loads the .env file located in the environment's current directory or its parents in sequence.
    // .env used only for development, so we discard error in all other cases.
    dotenvy::dotenv().ok();

    // Tries to load tracing config from environment (RUST_LOG) or uses "debug".
    telemetry::setup_tracing();

    // Parse configuration from the environment.
    // This will exit with a help message if something is wrong.
    tracing::debug!("Initializing configuration");
    let cfg = Configuration::new();

    // Initialize db pool.
    tracing::debug!("Initializing db pool");
    let db = Db::new(&cfg.db_dsn, cfg.db_pool_max_size)
        .await
        .expect("Failed to initialize db");

    tracing::debug!("Running migrations");
    db.migrate().await.expect("Failed to run migrations");

    // Initialize calendar state
    let calendar = Arc::new(RwLock::new(CalendarMap::new()));
    let weather = Arc::new(RwLock::new(Default::default()));
    let now_odt = OffsetDateTime::now_utc();
    let last_update = Arc::new(RwLock::new(PrimitiveDateTime::new(
        now_odt.date(),
        now_odt.time(),
    )));

    // Spin up our server.
    tracing::info!("Starting server on {}", cfg.listen_address);
    let listener = TcpListener::bind(&cfg.listen_address)
        .await
        .expect("Failed to bind address");
    let router = server::router(
        cfg.clone(),
        db,
        calendar.clone(),
        weather.clone(),
        last_update.clone(),
    );
    let http_task = async {
        axum::serve(listener, router)
            .await
            .expect("Failed to start server");
    };

    // Spin up cron
    let cron = cron::setup(
        cfg.clone(),
        calendar.clone(),
        weather.clone(),
        last_update.clone(),
    )
    .await
    .expect("Failed to start Cron");
    let cron_task = async {
        cron.start().await.expect("Failed to run Cron");
    };

    let _res = tokio::join!(http_task, cron_task);
}
