extern crate rand;

use std::sync::Arc;

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

mod routes;

#[tokio::main]
async fn main() {
    let config = WorldConfig {
        start_energy: 40,
        dead_energy: 20,
        split_behaviour: |i| {
            if i > 80 {
                Ok(i / 2)
            } else {
                Err(())
            }
        },
        light_behaviour: |i| 7usize.saturating_sub(i / 2),
        mutation_chance: 15,
        max_cell_size: 400,
    };

    let state = Arc::new(parking_lot::Mutex::new({
        let mut world = World::empty::<40, 20>(config);
        world.populate(200).unwrap();
        ServerState {
            paused: false,
            target_tps: 0,
            stats: SpeedMeasure::new(),
            world,
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

    let routes = routes::build_routes(state);
    println!("http://127.0.0.1:8000");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
