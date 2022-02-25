extern crate rand;

use std::sync::Arc;

use std::time::Duration;
use tokio::task;

use cells::world::World;
mod api;
mod cells;

mod state;
use state::ServerState;

use crate::cells::world::WorldConfig;

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
            tps: 1000000,
            world,
        }
    }));

    {
        let state = state.clone();

        task::spawn(async move {
            let mut tps = 1000000;
            loop {
                tokio::time::sleep(Duration::from_micros(1000000 / tps)).await;
                let mut state = state.lock();

                if !state.paused {
                    let world = &mut state.world;
                    world.tick();
                }
                tps = state.tps;
            }
        })
    };

    let routes = routes::build_routes(state);
    println!("http://127.0.0.1:8000");
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

/*fn main() {
    let mut world: World = World::empty::<5, 5>(100);
    world.populate(10).unwrap();

    for i in 0..10 {
        world.tick();

        println!("{i}\n{}", world);
    }
}*/
