use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}
