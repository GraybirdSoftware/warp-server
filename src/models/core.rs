use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct User { 
    pub created_at: String,
    pub email: String,
    pub id: i64,
    pub role: String,
    pub username: String
}


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
