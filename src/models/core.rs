use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct User { 
    created_at: String,
    email: String,
    id: i32,
    role: String,
    username: String
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
