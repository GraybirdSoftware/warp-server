//! Free-text search across sources, functions, types and symbols.

use actix_web::{HttpRequest, HttpResponse, get, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    db::{Db, json_strings, like_contains, uid},
    error::{ApiError, ApiResult},
    models::{
        request::SearchQuery,
        response::{SearchIndexRow, SearchResponse},
    },
};

const KINDS: [&str; 4] = ["source", "function", "type", "symbol"];
const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 1000;

struct Filters {
    term: Option<String>,
    function_guid: Option<String>,
    commit_id: Option<i64>,
    source_id: Option<String>,
    tags_json: Option<String>,
    retrieve_data: bool,
}

impl Filters {
    fn from_query(q: &SearchQuery) -> Self {
        Self {
            term: q
                .q
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(like_contains),
            function_guid: q.function_guid.as_ref().map(uid),
            commit_id: q.commit_id,
            source_id: q.source_id.as_ref().map(uid),
            tags_json: q
                .source_tags
                .as_ref()
                .filter(|t| !t.is_empty())
                .map(json_strings),
            retrieve_data: q.retrieve_data.unwrap_or(false),
        }
    }

    /// A kind is skipped when a filter it cannot satisfy is present.
    fn applies_to(&self, kind: &str) -> bool {
        match kind {
            "source" => self.function_guid.is_none() && self.commit_id.is_none(),
            "type" => self.function_guid.is_none(),
            "symbol" => {
                self.function_guid.is_none()
                    && self.commit_id.is_none()
                    && self.source_id.is_none()
                    && self.tags_json.is_none()
            }
            _ => true,
        }
    }

    fn push_term(&self, qb: &mut QueryBuilder<'_, Sqlite>, column: &str) {
        if let Some(term) = &self.term {
            qb.push(format!(" AND {column} LIKE "))
                .push_bind(term.clone())
                .push(" ESCAPE '\\'");
        }
    }

    fn push_tags(&self, qb: &mut QueryBuilder<'_, Sqlite>, source_column: &str) {
        if let Some(tags) = &self.tags_json {
            qb.push(format!(
                " AND EXISTS (SELECT 1 FROM source_tags st WHERE st.source_id = {source_column} \
                 AND st.tag IN (SELECT value FROM json_each("
            ))
            .push_bind(tags.clone())
            .push(")))");
        }
    }

    fn push_source(&self, qb: &mut QueryBuilder<'_, Sqlite>, column: &str) {
        if let Some(source_id) = &self.source_id {
            qb.push(format!(" AND {column} = "))
                .push_bind(source_id.clone());
        }
    }

    fn push_commit(&self, qb: &mut QueryBuilder<'_, Sqlite>, column: &str) {
        if let Some(commit_id) = self.commit_id {
            qb.push(format!(" AND {column} = ")).push_bind(commit_id);
        }
    }
}

/// Push the UNION ALL body producing the `SearchIndexRow` columns.
fn push_union(qb: &mut QueryBuilder<'_, Sqlite>, kinds: &[&str], f: &Filters) {
    let mut first = true;
    for kind in kinds.iter().copied().filter(|k| f.applies_to(k)) {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        match kind {
            "source" => {
                qb.push(
                    "SELECT 'source' AS kind, s.id AS id, s.name AS name, s.id AS source_id, \
                     NULL AS commit_id, s.created_at AS created_at, NULL AS function_guid, \
                     NULL AS symbol_id, NULL AS data FROM source s WHERE 1=1",
                );
                f.push_term(qb, "s.name");
                f.push_source(qb, "s.id");
                f.push_tags(qb, "s.id");
            }
            "function" => {
                qb.push(
                    "SELECT 'function' AS kind, CAST(f.id AS TEXT) AS id, sym.name AS name, \
                     f.source_id AS source_id, f.commit_id AS commit_id, f.created_at AS created_at, \
                     f.guid AS function_guid, f.symbol_id AS symbol_id, ",
                );
                qb.push(if f.retrieve_data {
                    "f.data AS data"
                } else {
                    "NULL AS data"
                });
                qb.push(" FROM functions f JOIN symbols sym ON sym.id = f.symbol_id WHERE 1=1");
                f.push_term(qb, "sym.name");
                if let Some(guid) = &f.function_guid {
                    qb.push(" AND f.guid = ").push_bind(guid.clone());
                }
                f.push_commit(qb, "f.commit_id");
                f.push_source(qb, "f.source_id");
                f.push_tags(qb, "f.source_id");
            }
            "type" => {
                qb.push(
                    "SELECT 'type' AS kind, t.id AS id, t.name AS name, t.source_id AS source_id, \
                     t.commit_id AS commit_id, t.created_at AS created_at, NULL AS function_guid, \
                     NULL AS symbol_id, ",
                );
                qb.push(if f.retrieve_data {
                    "t.data AS data"
                } else {
                    "NULL AS data"
                });
                qb.push(" FROM types t WHERE 1=1");
                f.push_term(qb, "t.name");
                f.push_commit(qb, "t.commit_id");
                f.push_source(qb, "t.source_id");
                f.push_tags(qb, "t.source_id");
            }
            "symbol" => {
                qb.push(
                    "SELECT 'symbol' AS kind, CAST(sym.id AS TEXT) AS id, sym.name AS name, \
                     NULL AS source_id, NULL AS commit_id, sym.created_at AS created_at, \
                     NULL AS function_guid, sym.id AS symbol_id, NULL AS data FROM symbols sym WHERE 1=1",
                );
                f.push_term(qb, "sym.name");
            }
            _ => unreachable!("kinds are validated before building the query"),
        }
    }
    if first {
        // Every kind was excluded by the filters: an always-empty result.
        qb.push(
            "SELECT 'none' AS kind, '' AS id, NULL AS name, NULL AS source_id, NULL AS commit_id, \
             '' AS created_at, NULL AS function_guid, NULL AS symbol_id, NULL AS data WHERE 0",
        );
    }
}

#[get("")]
async fn search(pool: web::Data<Db>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let query: SearchQuery = serde_qs::from_str(req.query_string())
        .map_err(|e| ApiError::BadRequest(format!("invalid query string: {e}")))?;

    let kinds: Vec<&str> = match query.kind.as_deref().map(str::trim) {
        None | Some("") => KINDS.to_vec(),
        Some(kind) if KINDS.contains(&kind) => vec![kind],
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "unknown kind '{other}'; expected one of {}",
                KINDS.join(", ")
            )));
        }
    };
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = query.offset.unwrap_or(0).max(0);
    let filters = Filters::from_query(&query);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM (");
    push_union(&mut count, &kinds, &filters);
    count.push(")");
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(
        "SELECT kind, id, name, source_id, commit_id, created_at, function_guid, symbol_id, data FROM (",
    );
    push_union(&mut select, &kinds, &filters);
    select
        .push(") ORDER BY kind, name, id LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let items: Vec<SearchIndexRow> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(SearchResponse {
        items,
        limit,
        offset,
        total,
    }))
}
