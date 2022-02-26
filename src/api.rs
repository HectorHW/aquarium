use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use warp::{
    hyper::{Response, StatusCode},
    reply::{Html, Json},
};

use crate::{cells::world::WorldCell, state::AMState};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum SerializedCell {
    Alive { energy: usize, minerals: usize },
    Dead { energy: usize, minerals: usize },
    Empty,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerializedWorld {
    cells: Vec<Vec<SerializedCell>>,
}

pub fn main_page(state: &AMState) -> Html<String> {
    let state = state.lock();
    let world = &state.world;

    let table_content = world
        .field
        .iter()
        .enumerate()
        .map(|(i, row)| {
            format!(
                "<tr>{}</tr>",
                row.iter()
                    .enumerate()
                    .map(|(j, item)| {
                        format!(
                            "<td>{}</td>",
                            match item {
                                WorldCell::Organism(o) => {
                                    format!(
                                        "<a href=/inspect/{i}/{j}>{} {}</a>",
                                        o.get_energy(),
                                        o.get_minerals()
                                    )
                                }
                                WorldCell::DeadBody(_e, m) => {
                                    format!("[{}]", m)
                                }

                                WorldCell::Empty => {
                                    " &nbsp; ".to_string()
                                }
                            }
                        )
                    })
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let table = format!("<table>{}</table>", table_content);

    let mut tera = Tera::default();
    tera.add_raw_template("index", include_str!("templates/index.html"))
        .unwrap();

    let mut context = Context::new();
    context.insert("table", &table);
    let step_count = state.world.total_steps.to_string();
    context.insert("steps", &step_count);
    context.insert(
        "tps",
        &format!(
            "tps: {} (measured at {:?})",
            state.stats.measured_tps, state.stats.measure_point
        ),
    );

    warp::reply::html(tera.render("index", &context).unwrap())
}

pub fn get_map(state: &AMState) -> Json {
    let state = state.lock();
    let world = &state.world;

    warp::reply::json(&SerializedWorld {
        cells: world
            .field
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        WorldCell::Empty => SerializedCell::Empty,
                        WorldCell::Organism(o) => SerializedCell::Alive {
                            energy: o.get_energy(),
                            minerals: o.get_minerals(),
                        },
                        WorldCell::DeadBody(energy, minerals) => SerializedCell::Dead {
                            energy: *energy,
                            minerals: *minerals,
                        },
                    })
                    .collect()
            })
            .collect(),
    })
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
    match state.world.field.get(i).and_then(|row| row.get(j)) {
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
    warp::reply::json(&state.stats.as_dict())
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

pub fn tick(state: &AMState) -> impl warp::Reply {
    let mut state = state.lock();
    let world = &mut state.world;
    world.tick();
    warp::reply()
}
