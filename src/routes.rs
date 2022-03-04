use warp::Filter;

use crate::{api, state::AMState};

pub fn build_routes(
    state: AMState,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + 'static {
    let react_app = warp::fs::dir("aquarium-front/build");

    let serve_world_json = warp::path!("world").map({
        let state = state.clone();
        move || api::get_map(&state)
    });

    let pause_world = warp::path!("pause").map({
        let state = state.clone();
        move || api::pause(&state)
    });

    let set_tps = warp::path!("set-tps").and(warp::body::json()).map({
        let state = state.clone();
        move |tps| api::set_tps(&state, tps)
    });

    let inspect = warp::path!("inspect" / usize / usize).map({
        let state = state.clone();
        move |i, j| api::inspect(&state, (i, j))
    });

    let stats = warp::path!("stats").map({
        let state = state.clone();
        move || api::stats(&state)
    });

    let spawn_random = warp::path!("spawn-random").and(warp::body::json()).map({
        let state = state.clone();
        move |n| api::spawn_random(&state, n)
    });

    let spawn_green = warp::path!("spawn-green").and(warp::body::json()).map({
        let state = state.clone();
        move |n| api::spawn_green(&state, n)
    });

    let tick = warp::path!("tick").map({
        let state = state.clone();
        move || api::tick(&state)
    });

    let set_setting = warp::path!("set-config").and(warp::body::json()).map({
        let state = state.clone();
        move |(k, v)| api::set_setting(&state, k, v)
    });

    let reset_world = warp::path!("reset").map({
        let state = state.clone();
        move || api::reset(&state)
    });

    let save_world = warp::path!("save-world").map({
        let state = state.clone();
        move || api::save_world(&state)
    });

    let load_world = warp::path!("load-world").and(warp::body::json()).map({
        let state = state.clone();
        move |data| api::load_world(&state, data)
    });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["*"])
        .allow_methods(vec!["*"]);

    let mappings = warp::get()
        .and(serve_world_json.or(inspect).or(stats).or(save_world))
        .or(warp::post().and(
            pause_world
                .or(set_tps)
                .or(spawn_random)
                .or(spawn_green)
                .or(tick)
                .or(set_setting)
                .or(reset_world)
                .or(load_world),
        ))
        .with(cors);

    react_app.or(mappings)
}
