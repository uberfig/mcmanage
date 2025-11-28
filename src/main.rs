use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{self, Data},
};
use mcmanage::{
    configuration::ConfigurationManager,
    server_runner::ServerRunner,
    versions::PackagesList,
    webui::{routes::get_api_routes, state::WebState},
};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = ConfigurationManager::new().await;

    if config.server_count().await == 0 {
        let packages = PackagesList::new().await;
        let latest = packages.get_latest_release();
        let info = latest.get_version_info().await;
        config
            .create_new_server("hello world".to_string(), info)
            .await;
    }
    let handle = ServerRunner::begin().await;
    config.start_all(handle.clone()).await;

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(WebState {
                config: config.clone(),
                runner_handle: handle.clone(),
            }))
            .service(get_api_routes())
            .service(hello)
            .service(echo)
            .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
