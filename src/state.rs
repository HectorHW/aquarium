use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::cells::world::World;

pub type AMState = Arc<MState>;

pub type MState = parking_lot::Mutex<ServerState>;

#[derive(Clone, Debug)]
pub struct SpeedMeasure {
    pub measured_tps: f64,
    pub measure_point: Instant,
}

impl SpeedMeasure {
    pub fn new() -> Self {
        Self {
            measured_tps: 0f64,
            measure_point: Instant::now(),
        }
    }

    pub fn take_measure(&mut self, world: &mut World) {
        let now = Instant::now();
        let time_span = now - self.measure_point;

        let current_step = world.measure_steps;
        // clip to 1 so we do not divide by zero
        let tick_delta = (current_step as f64).max(1.0);
        self.measured_tps = tick_delta / time_span.as_secs_f64();
        self.measure_point = now;
        world.measure_steps = 0;
    }

    pub fn as_dict(&self) -> HashMap<&'static str, String> {
        let mut map = HashMap::new();

        map.insert("measured_tps", format!("{}", self.measured_tps));
        map.insert("measure_point", format!("{:?}", self.measure_point));

        map
    }
}

pub struct ServerState {
    pub paused: bool,
    pub target_tps: u64,
    pub stats: SpeedMeasure,
    pub world: World,
    pub password: String,
}

impl ServerState {
    pub fn take_measure(&mut self) {
        self.stats.take_measure(&mut self.world)
    }
}
