use std::fmt::Display;

use rand::{distributions::Bernoulli, thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::cells::code::OpCode;

use super::{
    code::{Program, CODE_SIZE},
    world::World,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

    pub fn inverse(&self) -> Direction {
        self.next_clockwise().next_clockwise()
    }
}

impl From<u8> for Direction {
    fn from(n: u8) -> Self {
        match n % 4 {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            _ => Direction::Left,
        }
    }
}

impl From<Direction> for u8 {
    fn from(d: Direction) -> Self {
        match d {
            Direction::Up => 0,
            Direction::Right => 1,
            Direction::Down => 2,
            Direction::Left => 3,
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Organism {
    registers: [u8; 16],
    code: Program,
    ip: usize,

    energy: usize,
    stored_minerals: usize,
    pub can_clone: bool,
}

fn random_registers() -> [u8; 16] {
    let mut res = [0; 16];
    thread_rng().fill(&mut res);
    res
}

///registers
/// 0 - result register - observing instructions will put result here
/// 1 - result2 register
/// 2 - directional register - will store current bot direction
/// 3 - random value - regenerated on every tick
/// 4 - depth register
/// 5 - minerals
/// 6 - energy
/// 7 - attack
impl Organism {
    pub fn random(energy: usize) -> Self {
        let code = Program::random_program();
        Organism {
            registers: [0; 16],
            can_clone: code
                .code
                .iter()
                .any(|gene| matches!(gene, OpCode::Clone(..))),
            code,
            energy,
            stored_minerals: 0,
            ip: 0,
        }
    }

    pub fn green(energy: usize) -> Self {
        let program = Program {
            code: [OpCode::Sythesize; CODE_SIZE],
        };
        Self::with_program(energy, 0, program)
    }

    fn with_program(energy: usize, minerals: usize, program: Program) -> Self {
        Organism {
            registers: [0; 16],
            can_clone: program.iter().any(|gene| matches!(gene, OpCode::Clone(..))),
            code: program,
            energy,
            stored_minerals: minerals,
            ip: 0,
        }
    }

    #[inline(always)]
    fn next_instruction(&mut self) {
        self.ip = (self.ip + 1) % self.code.len();
    }

    #[inline(always)]
    fn jump(&mut self, delta: usize) {
        self.ip = (self.ip + delta) % self.code.len();
    }

    #[inline(always)]
    pub fn tick(&mut self, world: &World, (i, j): (usize, usize)) -> Option<OrganismAction> {
        self.registers[3] = thread_rng().gen();
        self.registers[4] = into_u8_fraction(i, world.get_height());
        self.registers[5] = into_u8_fraction(self.get_minerals(), world.config.max_minerals);
        self.registers[6] = into_u8_fraction(self.get_energy(), world.config.max_cell_size);

        if self.energy == 0 {
            return Some(OrganismAction::Die);
        }

        for _ in 0..16 {
            match self.code[self.ip] {
                OpCode::LoadInt(n) => {
                    self.next_instruction();
                    *self.result_register() = n;
                }
                OpCode::CopyRegisters(params) => {
                    self.next_instruction();
                    let (from, to) = params.unwrap();
                    self.registers[to] = self.registers[from];
                }
                OpCode::MoveRelative => {
                    self.next_instruction();
                    return Some(OrganismAction::TryMove(self.get_direction()));
                }
                OpCode::LookRelative => {
                    self.next_instruction();
                    let direction = self.get_direction();
                    let world_cell = world.look_relative((i, j), direction);
                    *self.result_register() = match world_cell {
                        Some(super::world::WorldCell::Empty) => 0,

                        Some(super::world::WorldCell::Organism(o)) => {
                            *self.result2_register() =
                                into_u8_fraction(o.get_energy(), world.config.max_cell_size);
                            1
                        }
                        Some(super::world::WorldCell::DeadBody(..)) => 2,
                        None => 255,
                    };
                }
                OpCode::Eat => {
                    self.next_instruction();
                    return Some(OrganismAction::TryEat(self.get_direction()));
                }
                OpCode::Sythesize => {
                    self.next_instruction();
                    let generated = world.get_light(i);
                    self.add_energy(generated);
                    return None;
                }

                OpCode::Add(addr) => {
                    self.next_instruction();
                    let (from, to) = addr.unwrap();
                    self.registers[from] = self.registers[from].wrapping_add(self.registers[to]);
                }
                OpCode::AddClip(addr) => {
                    self.next_instruction();
                    let (from, to) = addr.unwrap();
                    self.registers[from] = self.registers[from].saturating_add(self.registers[to]);
                }
                OpCode::SubClip(addr) => {
                    self.next_instruction();
                    let (from, to) = addr.unwrap();
                    self.registers[from] = self.registers[from].saturating_sub(self.registers[to]);
                }
                OpCode::Flip(addr) => {
                    self.next_instruction();
                    let addr = addr.unwrap();
                    self.registers[addr] = if self.registers[addr] != 0 { 1 } else { 0 };
                }
                OpCode::JumpUnconditional(shift) => {
                    self.jump(shift as usize);
                }
                OpCode::SkipZero(addr) => {
                    if self.registers[addr.unwrap()] == 0 {
                        self.jump(2);
                    } else {
                        self.next_instruction();
                    }
                }
                OpCode::Clone(inherit_rate) => {
                    self.next_instruction();
                    let child_energy = usize::max(
                        world.config.start_energy,
                        self.energy * inherit_rate as usize / 256usize / 2,
                    );

                    let child_minerals =
                        self.stored_minerals * inherit_rate as usize / 256usize / 2;

                    return Some(OrganismAction::TryClone(
                        child_energy,
                        child_minerals,
                        self.get_direction(),
                    ));
                }
                OpCode::Compare => {
                    self.next_instruction();
                    let direction = self.get_direction();
                    let world_cell = world.look_relative((i, j), direction);
                    *self.result_register() = match world_cell {
                        Some(super::world::WorldCell::Organism(other)) => {
                            *self.result2_register() =
                                into_u8_fraction(other.get_energy(), world.config.max_cell_size);

                            self.code
                                .iter()
                                .zip(other.code.iter())
                                .filter(|(a, b)| a != b)
                                .count()
                                .max(255) as u8
                        }
                        Some(super::world::WorldCell::DeadBody(..)) => 255,
                        _ => 0,
                    };
                }

                OpCode::UseMinerals => {
                    self.next_instruction();
                    let mineral_energy =
                        (*self.result_register() as usize).min(self.stored_minerals);
                    self.add_energy(mineral_energy);
                    self.stored_minerals -= mineral_energy;
                    return None;
                }
                OpCode::Share => {
                    self.next_instruction();
                    let share_value = usize::min(*self.result_register() as usize, self.energy);
                    self.energy -= share_value;
                    return Some(OrganismAction::ShareEnergy(
                        share_value,
                        self.get_direction(),
                    ));
                }

                OpCode::ShareMinerals => {
                    self.next_instruction();
                    let share_value =
                        usize::min(*self.result_register() as usize, self.stored_minerals);
                    self.stored_minerals -= share_value;
                    return Some(OrganismAction::ShareMinerals(
                        share_value,
                        self.get_direction(),
                    ));
                }
            };
        }
        None
    }

    pub fn add_energy(&mut self, energy: usize) {
        self.energy += energy;
    }

    pub fn decrease_energy(&mut self, energy: usize) {
        self.energy = self.energy.saturating_sub(energy);
    }

    pub fn add_minerals(&mut self, minerals: usize, limit: usize) {
        self.stored_minerals = (self.stored_minerals + minerals).min(limit);
    }

    #[inline(always)]
    fn get_direction(&self) -> Direction {
        self.registers[2].into()
    }

    #[inline(always)]
    fn result_register(&mut self) -> &mut u8 {
        &mut self.registers[0]
    }

    #[inline(always)]
    fn result2_register(&mut self) -> &mut u8 {
        &mut self.registers[1]
    }

    pub fn get_energy(&self) -> usize {
        self.energy
    }

    pub fn get_minerals(&self) -> usize {
        self.stored_minerals
    }

    pub fn register_attack(&mut self, direction: Direction) {
        self.registers[7] = direction.into();
    }

    pub fn split_off<F: FnOnce() -> Box<Organism>>(
        &mut self,
        allocation: F,
        energy: usize,
        minerals: usize,
        mutation_chance: usize,
    ) -> Option<Box<Organism>> {
        if self.energy >= energy * 2 {
            let mut alloc = allocation();

            let child_program = self.code.clone_lossy(mutation_chance);
            let bot = Self::with_program(energy, minerals, child_program);

            *alloc.as_mut() = bot;

            self.energy -= energy;
            self.stored_minerals -= minerals;
            Some(alloc)
        } else {
            None
        }
    }

    pub fn age(&mut self, aging_mutation_chance: &Bernoulli) {
        self.code.break_with_chance(aging_mutation_chance);
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

impl Default for Organism {
    fn default() -> Self {
        Organism::green(20)
    }
}

#[inline(always)]
fn into_u8_fraction(value: usize, divisor: usize) -> u8 {
    ((value * 255usize) / divisor).clamp(0, 255) as u8
}
