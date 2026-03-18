use actix_web::{
    App, HttpResponse, HttpServer, Responder,
    dev::Service,
    get, post,
    web::{self, service},
};

mod handlers;
mod routes;

async fn introspection_handler_text(
    tree: web::Data<actix_web::introspection::IntrospectionTree>,
) -> impl Responder {
    let report = tree.report_as_text();
    HttpResponse::Ok().content_type("text/plain").body(report)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/introspection").route(web::get().to(introspection_handler_text)))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/v1")
                            .service(web::scope("auth"))
                            .service(web::scope("commits"))
                            .service(web::scope("files"))
                            .service(web::scope("functions"))
                            .service(web::scope("search"))
                            .service(web::scope("sources"))
                            .service(web::scope("status")
                                    .route("", web::get().to(handlers::status::status_handler))
                                )
                            .service(web::scope("symbols"))
                            .service(web::scope("targets"))
                            .service(web::scope("types"))
                            .service(web::scope("users"))
                    )
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
