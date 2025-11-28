use actix_web::{
    HttpRequest, HttpResponse, Responder, get,
    http::{StatusCode, header::ContentType},
    post,
    web::{self, Data},
};
use lazy_static::lazy_static;
use tera::{Context, Tera};

use crate::{
    versions::PackagesList,
    webui::{
        auth::Info,
        state::{NewServer, WebState},
    },
};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "new_server.html",
            include_str!("../../webui/templates/new_server.html"),
        )
        .expect("Failed to add raw template");
        tera
    };
}

#[get("/new")]
async fn new_server(req: HttpRequest, state: Data<WebState>) -> impl Responder {
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
        println!("new server login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };

    let packages = PackagesList::new().await;
    let latest = packages.get_latest_release();

    let mut context = Context::new();
    context.insert("latest", &latest.id);
    context.insert("versions", &packages.versions);
    let body = TEMPLATES
        .render("new_server.html", &context)
        .expect("failed to render");

    HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::html())
        .body(body)
}

#[post("/new")]
async fn create_new_server(
    req: HttpRequest,
    state: Data<WebState>,
    web::Form(form): web::Form<NewServer>,
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
        println!("new server login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };

    state.create_new_server(form).await;

    HttpResponse::SeeOther()
        .insert_header(("Location", "/dash"))
        .body("success")
}
