use std::vec;

use serde::{Deserialize, Serialize};

use crate::cells::world::{World, WorldCellInner};

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
    let mut cells = vec![];

    for i in 0..world.field.get_height() {
        let mut row = vec![];

        for j in 0..world.field.get_width() {
            let cell = match &world.field[(i, j)] {
                WorldCellInner::Empty => SerializedCell::Empty,
                WorldCellInner::Organism(o) => SerializedCell::Alive {
                    energy: o.get_energy(),
                    minerals: o.get_minerals(),
                },
                WorldCellInner::DeadBody(energy, minerals) => SerializedCell::Dead {
                    energy: *energy,
                    minerals: *minerals,
                },
            };
            row.push(cell);
        }
        cells.push(row);
    }

    SerializedWorld { cells }
}
