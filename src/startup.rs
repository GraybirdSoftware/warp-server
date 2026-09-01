use std::net::TcpListener;

use actix_multipart::form::MultipartFormConfig;
use actix_web::{App, HttpResponse, HttpServer, Responder, Scope, dev::Server, middleware, web};

use crate::{
    config::Config,
    db::Db,
    handlers::{
        auth, commits, files, functions, search, sources, status, symbols, targets, types, users,
    },
};

/// JSON bodies carry base64 uploads of up to 64 MiB (~86 MiB encoded).
const JSON_LIMIT: usize = 96 * 1024 * 1024;
const MULTIPART_LIMIT: usize = 66 * 1024 * 1024;

async fn introspection_handler_text(
    tree: web::Data<actix_web::introspection::IntrospectionTree>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(tree.report_as_text())
}

/// Everything under `/api/v1`.
pub fn api_v1() -> Scope {
    web::scope("/api/v1")
        .service(
            web::scope("/auth")
                .service(auth::login)
                .service(auth::logout)
                .service(auth::get_callback),
        )
        .service(
            web::scope("/commits")
                .service(commits::query)
                .service(commits::get_commit)
                .service(commits::delete_commit),
        )
        .service(
            web::scope("/files")
                .service(files::push_file_multipart)
                .service(files::push_file_json),
        )
        .service(
            web::scope("/functions")
                .service(functions::query)
                .service(functions::query_source)
                .service(functions::get_functions_data)
                .service(functions::get_function)
                .service(functions::get_function_comments)
                .service(functions::get_function_constraints)
                .service(functions::get_function_data)
                .service(functions::get_function_symbol)
                .service(functions::get_function_type),
        )
        .service(web::scope("/search").service(search::search))
        .service(
            web::scope("/sources")
                .service(sources::create_source)
                .service(sources::query)
                .service(sources::get_source)
                .service(sources::update_source)
                .service(sources::delete_source)
                .service(sources::get_source_tags)
                .service(sources::get_source_users),
        )
        .service(web::scope("/status").service(status::get_status))
        .service(
            web::scope("/symbols")
                .service(symbols::query)
                .service(symbols::get_symbol),
        )
        .service(
            web::scope("/targets")
                .service(targets::query)
                .service(targets::get_query)
                .service(targets::get_target),
        )
        .service(
            web::scope("/types")
                .service(types::query)
                .service(types::get_types_data)
                .service(types::get_types)
                .service(types::get_types_data_by_id),
        )
        .service(
            web::scope("/users")
                .service(
                    web::scope("/me")
                        .service(users::me::get_user)
                        .service(users::me::get_keys)
                        .service(users::me::create_key),
                )
                .service(users::query)
                .service(users::create_user)
                .service(users::get_user)
                .service(users::delete_user)
                .service(users::get_keys)
                .service(users::create_key)
                .service(users::change_role)
                .service(users::change_username),
        )
}

pub fn run(listener: TcpListener, pool: Db, config: Config) -> std::io::Result<Server> {
    // Shared state must be wrapped in `Data` once, outside the factory closure.
    let pool = web::Data::new(pool);
    let config = web::Data::new(config);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(crate::middleware::SniffGzipBody)
            // `/api/v1/sources/{id}/` and `/api/v1/files/` are spelled with a
            // trailing slash in the upstream OpenAPI document.
            .wrap(middleware::NormalizePath::trim())
            .app_data(pool.clone())
            .app_data(config.clone())
            .app_data(web::JsonConfig::default().limit(JSON_LIMIT))
            .app_data(
                MultipartFormConfig::default()
                    .total_limit(MULTIPART_LIMIT)
                    .memory_limit(MULTIPART_LIMIT),
            )
            .service(
                web::resource("/introspection").route(web::get().to(introspection_handler_text)),
            )
            .service(api_v1())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
