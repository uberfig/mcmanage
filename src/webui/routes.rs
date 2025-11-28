use crate::webui::{auth::{login, login_page, signup, signup_page}, dash::dash, new::new_server};

pub fn get_api_routes() -> actix_web::Scope {
    actix_web::web::scope("")
        .service(login)
        .service(login_page)
        .service(signup_page)
        .service(signup)
        .service(dash)
        .service(new_server)
}
