use std::{collections::HashMap, sync::Arc, time::Instant};

use num_bigint::BigUint;

use num::{cast::ToPrimitive, CheckedSub};

use crate::cells::world::World;

pub type AMState = Arc<parking_lot::Mutex<ServerState>>;

#[derive(Clone, Debug)]
pub struct SpeedMeasure {
    pub measured_tps: f64,
    pub measure_point: Instant,
    previous_step: BigUint,
}

impl SpeedMeasure {
    pub fn new() -> Self {
        Self {
            measured_tps: 0f64,
            measure_point: Instant::now(),
            previous_step: BigUint::from(0usize),
        }
    }

    pub fn take_measure(&mut self, world: &World) {
        let now = Instant::now();
        let time_span = now - self.measure_point;

        let current_step = world.total_steps.clone();
        let tick_delta = (current_step.checked_sub(&self.previous_step))
            .and_then(|x| x.to_f64())
            .unwrap_or(0f64);
        self.measured_tps = tick_delta / time_span.as_secs_f64();
        self.measure_point = now;
        self.previous_step = current_step;
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
}

impl ServerState {
    pub fn take_measure(&mut self) {
        self.stats.take_measure(&self.world)
    }
}
