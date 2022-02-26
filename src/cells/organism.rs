use std::fmt::Display;

use rand::{thread_rng, Rng};

use crate::cells::code::OpCode;

use super::{code::Program, world::World};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[allow(dead_code)]
impl Direction {
    pub fn next_clockwise(&self) -> Direction {
        match self {
            Direction::Up => Direction::Right,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
            Direction::Right => Direction::Down,
        }
    }

    pub fn as_shift(&self) -> (isize, isize) {
        match self {
            Direction::Up => (-1, 0),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1),
        }
    }
}

impl From<u8> for Direction {
    fn from(n: u8) -> Self {
        match n % 4 {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OrganismAction {
    TryMove(Direction),
    TryEat(Direction),
    Die,
    TryClone(usize, usize, Direction),
    ShareEnergy(usize, Direction),
    ShareMinerals(usize, Direction),
}

#[derive(Copy, Clone, Debug)]
pub struct Organism {
    registers: [u8; 16],
    code: Program,
    ip: usize,

    energy: usize,
    stored_minerals: usize,
    pub can_clone: bool,
}

impl Organism {
    pub fn random(energy: usize) -> Self {
        let code = Program::random_program();
        Organism {
            ///registers
            /// 0 - result register - observing instructions will put result here
            /// 1 - directional register - will store current bot direction
            /// 2 - random value - regenerated on every tick
            /// 3..15 - unassigned
            registers: [0; 16],
            can_clone: code.0.iter().any(|gene| matches!(gene, OpCode::Clone(..))),
            code,
            energy,
            stored_minerals: 0,
            ip: 0,
        }
    }

    pub fn green(energy: usize) -> Self {
        let program = Program([OpCode::Sythesize; 256]);
        Self::with_program(energy, 0, program)
    }

    fn with_program(energy: usize, minerals: usize, program: Program) -> Self {
        Organism {
            ///registers
            /// 0 - result register - observing instructions will put result here
            /// 1 - directional register - will store current bot direction
            /// 2 - random value - regenerated on every tick
            /// 3..15 - unassigned
            registers: [0; 16],
            can_clone: program
                .0
                .iter()
                .any(|gene| matches!(gene, OpCode::Clone(..))),
            code: program,
            energy,
            stored_minerals: minerals,
            ip: 0,
        }
    }

    #[inline]
    pub fn tick(&mut self, world: &World, (i, j): (usize, usize)) -> Option<OrganismAction> {
        self.registers[2] = thread_rng().gen();
        self.registers[3] = i.min(255usize) as u8;
        self.registers[4] = self.get_minerals().min(255usize) as u8;
        self.registers[5] = self.get_energy().min(255usize) as u8;

        if self.energy == 0 {
            return Some(OrganismAction::Die);
        }

        self.energy -= 1;

        let res = match self.code.0[self.ip] {
            OpCode::LoadInt(n) => {
                *self.result_register() = n;
                self.ip += 1;
                None
            }
            OpCode::CopyRegisters(params) => {
                let (from, to) = params.unwrap();
                self.registers[to] = self.registers[from];
                self.ip += 1;
                None
            }
            OpCode::MoveRelative => {
                self.ip += 1;
                Some(OrganismAction::TryMove(self.get_direction()))
            }
            OpCode::LookRelative(addr) => {
                self.ip += 1;
                let direction = self.get_direction();
                let world_cell = world.look_relative((i, j), direction);
                self.registers[addr.unwrap()] = match world_cell {
                    Some(super::world::WorldCell::Empty) => 0,

                    Some(super::world::WorldCell::Organism(_)) => 1,
                    Some(super::world::WorldCell::DeadBody(..)) => 2,
                    None => 255,
                };
                None
            }
            OpCode::Eat => {
                self.ip += 1;
                Some(OrganismAction::TryEat(self.get_direction()))
            }
            OpCode::Sythesize => {
                self.ip += 1;
                let generated = world.get_light(i);
                self.energy += generated;
                None
            }

            OpCode::CollectMinerals => {
                self.ip += 1;
                let generated = world.get_minerals(i);
                self.stored_minerals =
                    (self.stored_minerals + generated).min(world.config.max_minerals);
                None
            }

            OpCode::Add(addr) => {
                self.ip += 1;
                let (from, to) = addr.unwrap();
                self.registers[from] += self.registers[to];
                None
            }
            OpCode::AddClip(addr) => {
                self.ip += 1;
                let (from, to) = addr.unwrap();
                self.registers[from] = self.registers[from].saturating_add(self.registers[to]);
                None
            }
            OpCode::SubClip(addr) => {
                self.ip += 1;
                let (from, to) = addr.unwrap();
                self.registers[from] = self.registers[from].saturating_sub(self.registers[to]);
                None
            }
            OpCode::Flip(addr) => {
                self.ip += 1;
                let addr = addr.unwrap();
                self.registers[addr] = if self.registers[addr] != 0 { 1 } else { 0 };
                None
            }
            OpCode::JumpUnconditional(shift) => {
                self.ip += shift as usize;
                None
            }
            OpCode::SkipZero(addr) => {
                if self.registers[addr.unwrap()] == 0 {
                    self.ip += 2;
                } else {
                    self.ip += 1;
                }
                None
            }
            OpCode::Clone(inherit_rate) => {
                let child_energy = usize::max(
                    world.config.start_energy,
                    self.energy * inherit_rate as usize / 256usize,
                );

                let child_minerals = self.stored_minerals * inherit_rate as usize / 256usize;

                Some(OrganismAction::TryClone(
                    child_energy,
                    child_minerals,
                    self.get_direction(),
                ))
            }
            OpCode::Compare(addr) => {
                self.ip += 1;
                let direction = self.get_direction();
                let world_cell = world.look_relative((i, j), direction);
                self.registers[addr.unwrap()] = match world_cell {
                    Some(super::world::WorldCell::Organism(other)) => {
                        self.code
                            .0
                            .iter()
                            .zip(other.code.0.iter())
                            .filter(|(a, b)| a != b)
                            .count()
                            .max(255) as u8
                    }
                    Some(super::world::WorldCell::DeadBody(..)) => 255,
                    _ => 0,
                };
                None
            }

            OpCode::UseMinerals(n) => {
                let mineral_energy =
                    (self.registers[n.unwrap()] as usize).min(self.stored_minerals);
                self.energy += mineral_energy;
                self.stored_minerals -= mineral_energy;
                None
            }
            OpCode::Share(addr) => {
                let share_value = usize::min(self.registers[addr.unwrap()] as usize, self.energy);
                self.energy -= share_value;
                Some(OrganismAction::ShareEnergy(
                    share_value,
                    self.get_direction(),
                ))
            }

            OpCode::ShareMinerals(addr) => {
                let share_value =
                    usize::min(self.registers[addr.unwrap()] as usize, self.stored_minerals);
                self.stored_minerals -= share_value;
                Some(OrganismAction::ShareMinerals(
                    share_value,
                    self.get_direction(),
                ))
            }
        };

        self.ip %= self.code.0.len();
        res
    }

    pub fn add_energy(&mut self, energy: usize) {
        self.energy += energy;
    }

    pub fn add_minerals(&mut self, minerals: usize, limit: usize) {
        self.stored_minerals = (self.stored_minerals + minerals).min(limit);
    }

    fn get_direction(&self) -> Direction {
        self.registers[1].into()
    }

    fn result_register(&mut self) -> &mut u8 {
        &mut self.registers[0]
    }

    pub fn get_energy(&self) -> usize {
        self.energy
    }

    pub fn get_minerals(&self) -> usize {
        self.stored_minerals
    }

    pub fn split_off(
        &mut self,
        energy: usize,
        minerals: usize,
        mutation_chance: usize,
    ) -> Option<Box<Organism>> {
        if self.energy > energy {
            let child_program = self.code.clone_lossy(mutation_chance);
            let child = Box::new(Self::with_program(energy, minerals, child_program));
            self.energy -= energy;
            self.stored_minerals -= minerals;
            Some(child)
        } else {
            None
        }
    }
}

impl Display for Organism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "
        energy: {}
        registers: {}
        ip: {}
        program: 
        {}
        ",
            self.get_energy(),
            self.registers
                .iter()
                .map(|reg| { format!("{}", reg) })
                .collect::<Vec<String>>()
                .join(", "),
            self.ip,
            self.code
        )
    }
}
