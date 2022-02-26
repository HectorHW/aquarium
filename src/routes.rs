use warp::Filter;

use crate::{api, state::AMState};

pub fn build_routes(
    state: AMState,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + 'static {
    let serve_page = warp::path::end().map({
        let state = state.clone();
        move || api::main_page(&state)
    });

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
        let state = state;
        move || api::stats(&state)
    });

    warp::get()
        .and(serve_page.or(serve_world_json).or(inspect).or(stats))
        .or(warp::post().and(pause_world.or(set_tps)))
}
