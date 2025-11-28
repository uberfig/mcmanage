use actix_web::{App, HttpServer, web::Data};
use mcmanage::{
    configuration::ConfigurationManager,
    server_runner::ServerRunner,
    webui::{routes::get_api_routes, state::WebState},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = ConfigurationManager::new().await;
    let handle = ServerRunner::begin().await;
    config.start_all(handle.clone()).await;

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(WebState {
                config: config.clone(),
                runner_handle: handle.clone(),
            }))
            .service(get_api_routes())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
