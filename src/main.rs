use actix_web::{
    App, HttpServer, http::header::LAST_MODIFIED, web::{self}
};

use crate::handlers::{
    auth, commits, files, functions, search, sources, status, symbols, targets, types,
};

mod handlers;
mod routes;
mod database;


#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let pool = database::create_pool().await;

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        .service(
                            web::scope("auth")
                                .service(auth::login)
                                .service(auth::logout)
                                .service(auth::get_callback),
                        )
                        .service(
                            web::scope("commits")
                                .service(commits::query)
                                .service(commits::get_commit)
                                .service(commits::delete_commit),
                        )
                        .service(
                            web::scope("files")
                                .service(files::files)
                                .service(files::json),
                        )
                        .service(
                            web::scope("functions")
                                .service(functions::query)
                                .service(functions::query_source)
                                .service(functions::get_function)
                                .service(functions::get_function_comments)
                                .service(functions::get_function_constraints)
                                .service(functions::get_function_data)
                                .service(functions::get_function_symbol)
                                .service(functions::get_function_type),
                        )
                        .service(web::scope("search").service(search::search))
                        .service(
                            web::scope("sources")
                                .service(sources::create_source)
                                .service(sources::query)
                                .service(sources::get_source)
                                .service(sources::update_source)
                                .service(sources::get_source_tags)
                                .service(sources::get_source_users),
                        )
                        .service(web::scope("status").service(status::get_status))
                        .service(
                            web::scope("symbols")
                                .service(symbols::query)
                                .service(symbols::get_symbol),
                        )
                        .service(
                            web::scope("targets")
                                .service(targets::query)
                                .service(targets::get_target),
                        )
                        .service(
                            web::scope("types")
                                .service(types::query)
                                .service(types::get_types)
                                .service(types::get_types_data),
                        )
                        .service(web::scope("users")),
                ),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
