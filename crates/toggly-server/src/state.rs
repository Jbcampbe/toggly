use toggly_core::db::Database;

pub struct AppState {
    pub db: Database,
    pub admin_api_key: String,
}

impl AppState {
    pub fn new(db: Database, admin_api_key: String) -> Self {
        Self { db, admin_api_key }
    }
}
