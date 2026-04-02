use std::sync::Arc;

use scylla::Session;

#[derive(Clone)]
pub struct AppState {
    pub session: Arc<Session>,
    pub keyspace: String,
}
