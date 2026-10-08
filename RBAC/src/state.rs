use std::sync::Arc;

use crate::surreal::Db;

pub struct AppState {
    pub db: Db,
}

impl AppState {
    pub fn new(db: Db) -> Arc<Self> {
        Arc::new(Self { db })
    }
}

pub type SharedState = Arc<AppState>;
