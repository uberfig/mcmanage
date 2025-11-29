use actix_web::{HttpRequest, HttpResponse, Responder, get, web::Data};

use crate::webui::{
    auth::{Info, login, login_page, signup, signup_page},
    commands::{command_dashboard, command_endpoint},
    dash::dash,
    new::{create_new_server, new_server},
    state::WebState,
    toggle_enabled::{set_disabled, set_enabled},
};

#[get("/")]
async fn home_redirector(req: HttpRequest, state: Data<WebState>) -> impl Responder {
    let user = if let Some(cookie) = req.cookie("auth") {
        if let Ok(user) = serde_json::from_str::<Info>(&cookie.value()) {
            match state
                .config
                .validate_password(user.username.clone(), user.password.clone())
                .await
            {
                Ok(_) => Some(user),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    };
    let Some(_user) = user else {
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };
    HttpResponse::TemporaryRedirect()
        .insert_header(("Location", "/dash"))
        .body("")
}

pub fn get_api_routes() -> actix_web::Scope {
    actix_web::web::scope("")
        .service(login)
        .service(login_page)
        .service(signup_page)
        .service(signup)
        .service(dash)
        .service(new_server)
        .service(create_new_server)
        .service(home_redirector)
        .service(set_disabled)
        .service(set_enabled)
        .service(command_dashboard)
        .service(command_endpoint)
}
