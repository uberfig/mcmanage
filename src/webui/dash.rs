use actix_web::{
    HttpRequest, HttpResponse, Responder, get,
    http::{StatusCode, header::ContentType},
    web::Data,
};
use lazy_static::lazy_static;
use tera::{Context, Tera};

use crate::{
    webui::{auth::Info, state::WebState},
};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_template("dash.html", include_str!("../../webui/templates/dash.html"))
            .expect("Failed to add raw template");
        tera
    };
}

#[get("/dash")]
async fn dash(req: HttpRequest, state: Data<WebState>) -> impl Responder {
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

    let mut context = Context::new();
    context.insert("servers", &state.config.get_servers().await);
    let body = TEMPLATES
        .render("dash.html", &context)
        .expect("failed to render");

    HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::html())
        .body(body)
}
