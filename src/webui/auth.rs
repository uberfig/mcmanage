use actix_web::{
    HttpResponse, Responder, Result,
    cookie::{self, Cookie, time::Duration},
    error::ErrorUnauthorized,
    get,
    http::{StatusCode, header::ContentType},
    post,
    web::{self, Data},
};
use cookie::time::OffsetDateTime;
use serde::{Deserialize, Serialize};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::{configuration::User, webui::state::WebState};

#[derive(Serialize, Deserialize)]
pub struct Info {
    pub username: String,
    pub password: String,
}

#[get("/login")]
async fn login_page() -> impl Responder {
    web::Html::new(include_str!("../../webui/login.html"))
}

#[post("/login")]
async fn login(web::Form(form): web::Form<Info>, state: Data<WebState>) -> Result<impl Responder> {
    if let Ok(_) = state
        .config
        .validate_password(form.username.clone(), form.password.clone())
        .await
    {
        let cookie = Cookie::build("auth", serde_json::to_string(&form).unwrap())
            .secure(true)
            .expires(OffsetDateTime::now_utc() + Duration::days(60))
            .finish();
        return Ok(HttpResponse::SeeOther()
            .insert_header(("Location", "/dash"))
            .cookie(cookie)
            .body("success"));
    }
    Err(ErrorUnauthorized("invalid username or password"))
}

#[get("/signup")]
async fn signup_page(state: Data<WebState>) -> impl Responder {
    if state.config.has_users().await {
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    }
    HttpResponse::build(StatusCode::OK)
        .content_type(ContentType::html())
        .body(include_str!("../../webui/signup.html"))
}

#[post("/signup")]
async fn signup(web::Form(form): web::Form<Info>, state: Data<WebState>) -> impl Responder {
    if state.config.has_users().await {
        return HttpResponse::TemporaryRedirect()
            .insert_header(("Location", "/login"))
            .body("");
    }
    let salt = SaltString::generate(&mut OsRng);
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2.hash_password(form.password.as_bytes(), &salt);
    match password_hash {
        Ok(hash) => {
            state
                .config
                .add_user(User {
                    username: form.username.clone(),
                    hashed_password: hash.to_string(),
                    is_admin: true,
                })
                .await;
            let cookie = Cookie::build("auth", serde_json::to_string(&form).unwrap())
                .secure(true)
                .expires(OffsetDateTime::now_utc() + Duration::days(60))
                .finish();
            return HttpResponse::SeeOther()
                .insert_header(("Location", "/dash"))
                .cookie(cookie)
                .body("success");
        }
        Err(_) => {
            return HttpResponse::BadRequest().body("invalid password");
        }
    }
}
