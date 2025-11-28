use crate::webui::{auth::Info, state::WebState};
use actix_web::{
    HttpRequest, HttpResponse, Responder, post,
    web::{self, Data},
};

#[post("disable/{id}")]
async fn set_disabled(
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
        println!("new server login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };

    state.disable_server(path.into_inner()).await;

    HttpResponse::SeeOther()
        .insert_header(("Location", "/dash"))
        .body("success")
}

#[post("enable/{id}")]
async fn set_enabled(
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
        println!("new server login failed");
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    };

    state.enable_server(path.into_inner()).await;

    HttpResponse::SeeOther()
        .insert_header(("Location", "/dash"))
        .body("success")
}
