use warp::{
    hyper::{Response, StatusCode},
    reply::Json,
};

use crate::{
    cells::world::{WorldCell, WorldField},
    serialization::store_world_shallow,
    state::AMState,
};

pub fn get_map(state: &AMState) -> Json {
    let state = state.lock();
    let world = &state.world;

    warp::reply::json(&store_world_shallow(world))
}

pub fn pause(state: &AMState) -> Response<String> {
    let mut state = state.lock();

    state.paused = !state.paused;

    Response::builder()
        .header(
            "pause-state",
            format!("{}", if state.paused { 1 } else { 0 }),
        )
        .body("".to_string())
        .unwrap()
}

pub fn set_tps(state: &AMState, tps: u64) -> impl warp::Reply {
    if !(0..=1000).contains(&tps) {
        return warp::reply::with_status(
            format!("invalid tps value {}", tps),
            StatusCode::BAD_REQUEST,
        );
    }

    let mut state = state.lock();
    state.target_tps = tps;

    warp::reply::with_status(format!("set tps to {tps}"), StatusCode::OK)
}

pub fn inspect(state: &AMState, (i, j): (usize, usize)) -> impl warp::Reply {
    let state = state.lock();
    match state.world.field.get((i, j)) {
        Some(cell) => warp::reply::with_status(
            match cell {
                WorldCell::Empty => "empty cell".to_string(),
                WorldCell::Organism(bot) => {
                    format!(
                        "
                    {}
                    ",
                        bot
                    )
                }
                WorldCell::DeadBody(..) => "dead body".to_string(),
            },
            StatusCode::OK,
        ),
        None => warp::reply::with_status(
            format!("({}, {}) out of bounds", i, j),
            StatusCode::BAD_REQUEST,
        ),
    }
}

pub fn stats(state: &AMState) -> Json {
    let state = state.lock();
    let mut stats = state.stats.as_dict();
    stats.insert(
        "is_paused",
        if state.paused { "1" } else { "0" }.to_string(),
    );
    warp::reply::json(&stats)
}

pub fn spawn_random(state: &AMState, bots: usize) -> impl warp::Reply {
    let mut state = state.lock();
    let world = &mut state.world;
    match world.populate_random(bots) {
        Ok(_) => warp::reply::with_status("".to_string(), StatusCode::CREATED),
        Err(n) => {
            warp::reply::with_status(format!("failed to add {} bots", n), StatusCode::CONFLICT)
        }
    }
}

pub fn spawn_green(state: &AMState, bots: usize) -> impl warp::Reply {
    let mut state = state.lock();
    let world = &mut state.world;
    match world.populate_green(bots) {
        Ok(_) => warp::reply::with_status("".to_string(), StatusCode::CREATED),
        Err(n) => {
            warp::reply::with_status(format!("failed to add {} bots", n), StatusCode::CONFLICT)
        }
    }
}

pub fn tick(state: &AMState) -> Json {
    let mut state = state.lock();
    let world = &mut state.world;
    world.tick();
    warp::reply::json(&store_world_shallow(world))
}

pub fn set_setting(state: &AMState, key: String, value: usize) -> impl warp::Reply {
    let mut state = state.lock();
    match key.as_str() {
        "mutation_chance" => {
            state.world.config.mutation_chance = value;
        }

        "max_cell_size" => {
            state.world.config.max_cell_size = value;
        }

        "max_minerals" => {
            state.world.config.max_cell_size = value;
        }

        other => {
            return warp::reply::with_status(
                format!("parameter not found: {}", other),
                StatusCode::BAD_REQUEST,
            );
        }
    }
    warp::reply::with_status("ok".to_string(), StatusCode::OK)
}

pub fn reset(state: &AMState) -> impl warp::Reply {
    let mut state = state.lock();
    let world = &mut state.world;
    world
        .field
        .inner
        .iter_mut()
        .for_each(|cell| *cell = WorldCell::Empty);
    warp::reply()
}

pub fn save_world(state: &AMState) -> Json {
    let state = state.lock();
    let world = &state.world;

    warp::reply::json(&world.field)
}

pub fn load_world(state: &AMState, data: WorldField) -> impl warp::Reply {
    let mut state = state.lock();
    state.world.field = data;
    warp::reply()
}
