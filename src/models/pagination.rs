use serde::{Deserialize, Serialize};

/// Body of `POST /auth/login`: where to send the browser after login.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NextUrl {
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedVec<T> {
    pub items: Vec<T>,
    pub total_pages: i64,
    pub total_results: i64,
}

pub const DEFAULT_LIMIT: i64 = 100;
/// The Binary Ninja plugin asks for up to 10 000 functions per query.
pub const MAX_LIMIT: i64 = 10_000;

/// Normalised `limit`/`page` pair. Pages are zero-based.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: i64,
    pub page: i64,
}

impl Pagination {
    pub fn new(limit: Option<i64>, page: Option<i64>) -> Self {
        Self {
            limit: limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
            page: page.unwrap_or(0).max(0),
        }
    }

    pub fn offset(&self) -> i64 {
        self.page.saturating_mul(self.limit)
    }

    pub fn wrap<T>(&self, items: Vec<T>, total_results: i64) -> PaginatedVec<T> {
        PaginatedVec {
            items,
            total_pages: (total_results + self.limit - 1) / self.limit,
            total_results,
        }
    }
}
