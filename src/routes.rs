use actix_web::Scope;

use crate::{api, state::AMState};

pub fn build_routes(state: AMState, base_path: &str) -> Scope {
    let react_app = actix_files::Files::new(".", "front/build").index_file("index.html");

    let state = actix_web::web::Data::from(state);

    let api = Scope::new("/api")
        .app_data(state)
        .service(api::get_map)
        .service(api::pause)
        .service(api::set_tps)
        .service(api::inspect)
        .service(api::stats)
        .service(api::spawn_random)
        .service(api::spawn_green)
        .service(api::tick)
        .service(api::set_setting)
        .service(api::reset)
        .service(api::save_world)
        .service(api::load_world);

    Scope::new(base_path)
        .service(api)
        .default_service(react_app)
}
