use axum::{Router, body::Body, http::Request, http::Response};
use sqlx::{Connection, Executor, PgConnection};
use std::sync::{Arc, Once};
use time::{OffsetDateTime, PrimitiveDateTime};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use server::{Configuration, Db, model::CalendarMap, telemetry};

static TRACING: Once = Once::new();

pub struct TestApp {
    pub router: Router,
    pub db: Db,
}

impl TestApp {
    pub async fn new() -> Self {
        // Loads the .env file located in the environment's current directory or its parents in sequence.
        // .env used only for development, so we discard error in all other cases.
        dotenvy::dotenv().ok();

        // Set port to 0 so tests can spawn multiple servers on OS assigned ports.
        unsafe { std::env::set_var("PORT", "0") };

        // Setup tracing. Once.
        TRACING.call_once(telemetry::setup_tracing);

        // Parse configuration from the environment.
        // This will exit with a help message if something is wrong.
        let cfg = Configuration::new();

        // Creates db with a random name for tests.
        let db_dsn = create_test_db(&cfg.db_dsn).await;
        // Initialize test db pool.
        let db = Db::new(&db_dsn, cfg.db_pool_max_size)
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

        let router = server::router(
            cfg.clone(),
            db.clone(),
            calendar.clone(),
            weather.clone(),
            last_update.clone(),
        );
        Self { db, router }
    }

    pub async fn request(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }
}

/// Creates db with a random name for tests.
pub async fn create_test_db(db_dsn: &str) -> String {
    let db_name =
        std::env::var("DATABASE_NAME").expect("Missing DATABASE_NAME environment variable");
    let db_dsn = db_dsn
        .strip_suffix(&db_name)
        .expect("Failed to remove db name from dsn_url");
    let randon_db_name = Uuid::now_v7().to_string();
    let db_url = format!("{}{}", &db_dsn, randon_db_name);
    let mut conn = PgConnection::connect(db_dsn)
        .await
        .expect("Failed to connect to Postgres");
    conn.execute(format!(r#"CREATE DATABASE "{}";"#, randon_db_name).as_str())
        .await
        .expect("Failed to create test database");
    db_url
}
