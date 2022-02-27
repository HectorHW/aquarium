use serde::{Deserialize, Serialize};

use crate::cells::world::{World, WorldCell};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SerializedCell {
    Alive { energy: usize, minerals: usize },
    Dead { energy: usize, minerals: usize },
    Empty,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedWorld {
    cells: Vec<Vec<SerializedCell>>,
}

pub fn store_world_shallow(world: &World) -> SerializedWorld {
    SerializedWorld {
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
    }
}
