use std::time::Instant;

use crate::{
    cells::world::{WorldCell, WorldField},
    serialization::store_world_shallow,
    state::MState,
};

use actix_web::{
    get, post,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};

#[get("/world")]
pub async fn get_map(state: Data<MState>) -> impl Responder {
    let state = state.lock();

    let world = &state.world;

    Json(store_world_shallow(world))
}

#[post("/human")]
pub async fn set_last_human(state: Data<MState>) -> impl Responder {
    let mut state = state.lock();
    state.last_human_request = Instant::now();
    HttpResponse::Ok()
}

#[post("/pause")]
pub async fn pause(state: Data<MState>) -> impl Responder {
    let mut state = state.lock();

    state.paused = !state.paused;

    HttpResponse::Ok()
        .append_header(("pause-state", format!("{}", i32::from(state.paused))))
        .body("")
}

#[get("/inspect/{i}/{j}")]
pub async fn inspect(state: Data<MState>, idx: Path<(usize, usize)>) -> impl Responder {
    //let i: usize = req.match_info().get("i");
    //let (i, j) = params.into_inner();
    let (i, j) = *idx;

    let state = state.lock();
    match state.world.field.get((i, j)) {
        Some(cell) => HttpResponse::Ok().body(match cell {
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
        }),
        None => HttpResponse::NotFound().body(format!("({}, {}) out of bounds", i, j)),
    }
}

#[get("/stats")]
pub async fn stats(state: Data<MState>) -> impl Responder {
    let state = state.lock();
    let mut stats = state.stats.as_dict();
    stats.insert(
        "is_paused",
        if state.paused { "1" } else { "0" }.to_string(),
    );
    Json(stats.clone())
}

#[post("/spawn-random")]
pub async fn spawn_random(state: Data<MState>, bots: Json<usize>) -> impl Responder {
    let mut state = state.lock();
    let world = &mut state.world;
    match world.populate_random(bots.0) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(n) => HttpResponse::Conflict().body(format!("failed to add {} bots", n)),
    }
}

#[post("/spawn-green")]
pub async fn spawn_green(state: Data<MState>, bots: Json<usize>) -> impl Responder {
    let mut state = state.lock();
    let world = &mut state.world;
    match world.populate_green(bots.0) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(n) => HttpResponse::Conflict().body(format!("failed to add {} bots", n)),
    }
}

#[post("/tick")]
pub async fn tick(state: Data<MState>) -> impl Responder {
    let mut state = state.lock();
    let world = &mut state.world;
    world.tick();
    Json(store_world_shallow(world))
}

#[post("/set-config/{key}")]
pub async fn set_setting(
    state: Data<MState>,
    key: Path<String>,
    value: Json<usize>,
) -> impl Responder {
    let mut state = state.lock();
    let key = key.into_inner();
    let value = value.0;
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
            return HttpResponse::BadRequest().body(format!("parameter not found: {}", other));
        }
    }
    HttpResponse::Ok().finish()
}

#[post("/reset")]
pub async fn reset(state: Data<MState>) -> impl Responder {
    let mut state = state.lock();
    let world = &mut state.world;
    world
        .field
        .inner
        .iter_mut()
        .for_each(|cell| *cell = WorldCell::Empty);
    HttpResponse::Ok()
}

#[get("/save-world")]
pub async fn save_world(state: Data<MState>) -> impl Responder {
    let state = state.lock();
    let world = &state.world;

    Json(world.field.clone())
}

#[post("/load-world")]
pub async fn load_world(state: Data<MState>, data: Json<WorldField>) -> impl Responder {
    let data = data.0;
    let mut state = state.lock();
    state.world.field = data;
    HttpResponse::Ok()
}

#[post("/auth")]
pub async fn auth(state: Data<MState>, password: Json<String>) -> impl Responder {
    let state = state.lock();
    if state.password == *password {
        HttpResponse::Ok().json(&state.secret)
    } else {
        HttpResponse::Unauthorized().finish()
    }
}
