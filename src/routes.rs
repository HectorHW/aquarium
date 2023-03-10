use actix_web::{dev::Service, web::Data, Scope};

use actix_web::error::ErrorUnauthorized;

use crate::{
    api,
    state::{AMState, MState},
};

pub fn build_routes(state: AMState, base_path: &str) -> Scope {
    let react_app = actix_files::Files::new(".", "front/build").index_file("index.html");

    let state = actix_web::web::Data::from(state);

    let api_protected = Scope::new("")
        .service(api::pause)
        .service(api::spawn_random)
        .service(api::spawn_green)
        .service(api::tick)
        .service(api::set_setting)
        .service(api::reset)
        .service(api::load_world)
        .wrap_fn(|req, srv| {
            let accepted = {
                let provided_token = req
                    .cookie("aquarium_auth_token")
                    .map(|c| c.value().to_owned())
                    .unwrap_or_else(|| "none".to_string());

                let state = req.app_data::<Data<MState>>().unwrap().lock();
                let expected_token = &state.secret;
                expected_token == &provided_token
            };

            let fut = srv.call(req);
            async move {
                if accepted {
                    fut.await
                } else {
                    Err(ErrorUnauthorized("wrong token"))
                }
            }
        });

    let api = Scope::new("/api")
        .app_data(state)
        .service(api::auth)
        .service(api::get_map)
        .service(api::set_last_human)
        .service(api::inspect)
        .service(api::stats)
        .service(api::save_world)
        .service(api_protected);

    Scope::new(base_path)
        .service(api)
        .default_service(react_app)
}
