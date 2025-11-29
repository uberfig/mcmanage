use actix_web::{
    HttpRequest, HttpResponse, Responder, get,
    http::{StatusCode, header::ContentType},
    post,
    web::{self, Data},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

use crate::webui::{auth::Info, state::WebState};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "commands.html",
            include_str!("../../webui/templates/commands.html"),
        )
        .expect("Failed to add raw template");
        tera
    };
}

#[get("/server/{id}")]
async fn command_dashboard(
    req: HttpRequest,
    state: Data<WebState>,
    path: web::Path<usize>,
) -> impl Responder {
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
            println!("bad cookie");
            None
        }
    } else {
        println!("no cookie");
        None
    };
    let Some(_user) = user else {
        println!("dash login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };
    let Some(server) = state.config.get_server(path.into_inner()).await else {
        return HttpResponse::build(StatusCode::NOT_FOUND).body("server not found");
    };
    let output = state
        .runner_handle
        .get_output(server.id)
        .await
        .unwrap_or("".to_string());

    let mut context = Context::new();
    context.insert("server", &server);
    context.insert("output", &output);
    let body = TEMPLATES
        .render("commands.html", &context)
        .expect("failed to render");

    HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::html())
        .body(body)
}

#[derive(Serialize, Deserialize)]
pub struct Command {
    pub command: String,
}

#[post("/command/{id}")]
async fn command_endpoint(
    req: HttpRequest,
    state: Data<WebState>,
    path: web::Path<usize>,
    web::Form(form): web::Form<Command>,
) -> impl Responder {
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
            println!("bad cookie");
            None
        }
    } else {
        println!("no cookie");
        None
    };
    let Some(_user) = user else {
        println!("dash login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };
    let Some(server) = state.config.get_server(path.into_inner()).await else {
        return HttpResponse::build(StatusCode::NOT_FOUND).body("server not found");
    };

    state.runner_handle.issue_command(server.id, form.command);

    HttpResponse::SeeOther()
        .insert_header(("Location", format!("/server/{}", server.id)))
        .body("success")
}
