use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::Type)]
pub enum Role {
    User,
    Admin,
}

#[derive(Serialize, Deserialize)]
pub enum Format {
    Json,
    Flatbuffer,
}
