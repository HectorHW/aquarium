use std::sync::Arc;

use crate::cells::world::World;

pub type AMState = Arc<parking_lot::Mutex<ServerState>>;

pub struct ServerState {
    pub paused: bool,
    pub tps: u64,
    pub world: World,
}
