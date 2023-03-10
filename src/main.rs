extern crate rand;

use std::sync::Arc;

use rand::distributions::Bernoulli;

use std::thread;
use std::time::Duration;
use tokio::task;

use cells::world::World;
mod api;
mod cells;

mod state;
use state::ServerState;

use crate::cells::world::WorldConfig;
use crate::state::SpeedMeasure;

mod cachealloc;
mod routes;
mod serialization;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = WorldConfig {
        start_energy: 40,
        dead_energy: 20,
        split_behaviour: |energy, minerals| {
            if energy > 200 {
                Ok((energy / 2, minerals / 2))
            } else {
                Err(())
            }
        },
        light_behaviour: |i| 3usize.saturating_sub(i / 10),
        mutation_chance: 1,
        aging_mutation_freq: Bernoulli::from_ratio(1, 1000).unwrap(),
        max_cell_size: 500,
        minerals_behaviour: |i| {
            let distance_from_bottom = 50 - i - 1;
            3usize.saturating_sub(distance_from_bottom / 10)
        },
        max_minerals: 100,
        attack_cost: 10,
    };

    let state = Arc::new(parking_lot::Mutex::new({
        let world = World::empty::<100, 50>(config);
        ServerState {
            paused: false,
            target_tps: 0,
            stats: SpeedMeasure::new(),
            world,
            password: "1234".to_string(),
        }
    }));

    {
        let state = state.clone();

        task::spawn_blocking(move || {
            let mut tps = 0;
            loop {
                if tps != 0 {
                    thread::sleep(Duration::from_millis(1000 / tps));
                }

                let mut state = state.lock();

                if !state.paused {
                    let world = &mut state.world;
                    world.tick();
                }
                tps = state.target_tps;
            }
        })
    };

    {
        let state = state.clone();
        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let mut state = state.lock();
                {
                    state.take_measure();
                }
            }
        })
    };

    println!("http://127.0.0.1:8000/aquarium");

    ctrlc::set_handler(move || {
        println!("got SIGINT, exiting");
        std::process::exit(0)
    })
    .unwrap();

    HttpServer::new({
        let state = state.clone();
        move || App::new().service(routes::build_routes(state.clone(), "aquarium"))
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
