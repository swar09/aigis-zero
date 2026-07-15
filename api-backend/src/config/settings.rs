use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub database_url_nodes: String,
    pub database_url_alerts: String,
    pub database_url_logs: String,
    pub db_pool_max_size: usize,
    pub kafka_brokers: String,
    pub kafka_consumer_group: String,
    pub fleet_grpc_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_secs: i64,
    pub admin_default_user: String,
    pub admin_default_password: String,
}

impl Settings {
    pub fn load_from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()?;

        let database_url_nodes = env::var("DATABASE_URL_NODES")
            .unwrap_or_else(|_| "postgres://edr:edrpassword@localhost:5433/edr_nodes".to_string());
        let database_url_alerts = env::var("DATABASE_URL_ALERTS")
            .unwrap_or_else(|_| "postgres://edr:edrpassword@localhost:5434/edr_alerts".to_string());
        let database_url_logs = env::var("DATABASE_URL_LOGS")
            .unwrap_or_else(|_| "postgres://edr:edrpassword@localhost:5432/edr_logs".to_string());

        let db_pool_max_size = env::var("DB_POOL_MAX_SIZE")
            .unwrap_or_else(|_| "16".to_string())
            .parse::<usize>()
            .unwrap_or(16);

        let kafka_brokers =
            env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
        let kafka_consumer_group =
            env::var("KAFKA_CONSUMER_GROUP").unwrap_or_else(|_| "edr-api-backend-live".to_string());

        let fleet_grpc_url =
            env::var("FLEET_GRPC_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            "super_secret_jwt_key_replace_in_production_32_bytes_min".to_string()
        });
        let jwt_expiration_secs = env::var("JWT_EXPIRATION_SECS")
            .unwrap_or_else(|_| "86400".to_string())
            .parse::<i64>()
            .unwrap_or(86400);

        let admin_default_user =
            env::var("ADMIN_DEFAULT_USER").unwrap_or_else(|_| "admin".to_string());
        let admin_default_password =
            env::var("ADMIN_DEFAULT_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        Ok(Self {
            host,
            port,
            database_url_nodes,
            database_url_alerts,
            database_url_logs,
            db_pool_max_size,
            kafka_brokers,
            kafka_consumer_group,
            fleet_grpc_url,
            jwt_secret,
            jwt_expiration_secs,
            admin_default_user,
            admin_default_password,
        })
    }
}
