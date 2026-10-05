use lettre::SmtpTransport;
use redis::aio::ConnectionManager;
use surrealdb::{Surreal, engine::remote::ws::Client};

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Surreal<Client>,
    pub redis: ConnectionManager,
    pub mailer: SmtpTransport,
    pub config: Config,
}
