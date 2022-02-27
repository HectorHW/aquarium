use std::{fmt::Display, mem};

use num_bigint::BigUint;
use rand::{prelude::SliceRandom, thread_rng};

use super::organism::{Direction, Organism, OrganismAction};

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub start_energy: usize,
    pub dead_energy: usize,
    pub split_behaviour: fn(usize, usize) -> Result<(usize, usize), ()>,
    pub light_behaviour: fn(usize) -> usize,
    pub minerals_behaviour: fn(usize) -> usize,
    pub mutation_chance: usize,
    pub max_cell_size: usize,
    pub max_minerals: usize,
}

#[derive(Clone, Debug)]
pub enum WorldCell {
    Empty,
    Organism(Box<Organism>),
    DeadBody(usize, usize),
}

pub struct World {
    pub field: Vec<Vec<WorldCell>>,
    iteration: usize,
    updates: Vec<Vec<usize>>,

    pub config: WorldConfig,
    pub total_steps: BigUint,
}

impl World {
    pub fn empty<const WIDTH: usize, const HEIGHT: usize>(config: WorldConfig) -> Self {
        let field = vec![vec![WorldCell::Empty; WIDTH]; HEIGHT];
        World {
            field,
            iteration: 1,
            updates: vec![vec![0; WIDTH]; HEIGHT],
            config,
            total_steps: BigUint::from(0usize),
        }
    }

    fn get_free_cells(&self) -> Vec<(usize, usize)> {
        let mut res = vec![];
        for (i, row) in self.field.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if matches!(cell, &WorldCell::Empty) {
                    res.push((i, j));
                }
            }
        }
        res
    }

    fn populate(
        &mut self,
        mut number_of_bots: usize,
        bot_factory: fn(usize) -> Organism,
    ) -> Result<(), usize> {
        let mut free_cells = self.get_free_cells();

        free_cells.shuffle(&mut thread_rng());

        for (i, j) in free_cells {
            if number_of_bots == 0 {
                break;
            }

            let organism = bot_factory(self.config.start_energy);

            self.field[i][j] = WorldCell::Organism(Box::new(organism));
            number_of_bots -= 1;
        }

        if number_of_bots > 0 {
            Err(number_of_bots)
        } else {
            Ok(())
        }
    }

    pub fn populate_green(&mut self, number_of_bots: usize) -> Result<(), usize> {
        self.populate(number_of_bots, Organism::green)
    }

    pub fn populate_random(&mut self, number_of_bots: usize) -> Result<(), usize> {
        self.populate(number_of_bots, Organism::random)
    }

    pub fn get_width(&self) -> usize {
        self.field[0].len()
    }

    pub fn get_height(&self) -> usize {
        self.field.len()
    }

    pub fn relative_shift(
        &self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<(usize, usize)> {
        match direction {
            Direction::Down if i == self.get_height() - 1 => {
                return None;
            }

            Direction::Up if i == 0 => {
                return None;
            }

            _ => {}
        }

        let shift = direction.as_shift();
        let (i, j) = (i as isize + shift.0, j as isize + shift.1);
        let (i, j) = (
            i + self.get_height() as isize,
            j + self.get_width() as isize,
        );
        let (i, j) = (
            i as usize % self.get_height(),
            j as usize % self.get_width(),
        );

        Some((i, j))
    }

    pub fn look_relative_mut(
        &mut self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<&mut WorldCell> {
        let (i, j) = self.relative_shift((i, j), direction)?;
        Some(&mut self.field[i][j])
    }

    pub fn look_relative(
        &self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<&WorldCell> {
        let (i, j) = self.relative_shift((i, j), direction)?;
        Some(&self.field[i][j])
    }

    pub fn get_light(&self, i: usize) -> usize {
        (self.config.light_behaviour)(i)
    }

    pub fn get_minerals(&self, i: usize) -> usize {
        (self.config.minerals_behaviour)(i)
    }

    #[inline]
    fn run_bot_prelude(&mut self, (i, _j): (usize, usize), bot: &mut Organism) {
        let minerals = self.get_minerals(i);
        bot.add_minerals(minerals, self.config.max_minerals)
    }

    #[inline]
    fn process_bot(&mut self, (i, j): (usize, usize), mut bot: Box<Organism>) {
        self.run_bot_prelude((i, j), bot.as_mut());

        match bot.tick(self, (i, j)) {
            Some(OrganismAction::TryEat(direction)) => {
                let dead_energy = self.config.dead_energy;
                match self.look_relative_mut((i, j), direction) {
                    Some(&mut WorldCell::Organism(ref mut other))
                        if bot.get_energy() >= other.get_energy() =>
                    {
                        let energy = other.get_energy();

                        bot.add_energy(energy.saturating_sub(dead_energy) / 2);
                        *self.look_relative_mut((i, j), direction).unwrap() = WorldCell::Empty;
                    }

                    Some(cell @ &mut WorldCell::DeadBody(..)) => {
                        let (energy, minerals) = match &cell {
                            WorldCell::DeadBody(e, m) => (*e, *m),
                            _ => unreachable!(),
                        };
                        *cell = WorldCell::Empty;
                        bot.add_energy(energy);
                        bot.add_minerals(minerals, self.config.max_minerals);
                    }

                    _any_other_case => {}
                }

                self.field[i][j] = WorldCell::Organism(bot);
            }
            Some(OrganismAction::TryMove(direction)) => {
                if let Some(WorldCell::Empty) = self.look_relative_mut((i, j), direction) {
                    let (new_i, new_j) = self.relative_shift((i, j), direction).unwrap();
                    self.field[new_i][new_j] = WorldCell::Organism(bot);
                    self.updates[new_i][new_j] = self.updates[new_i][new_j].wrapping_add(1);
                } else {
                    self.field[i][j] = WorldCell::Organism(bot);
                }
            }

            Some(OrganismAction::Die) => {
                self.field[i][j] = WorldCell::DeadBody(self.config.dead_energy, bot.get_minerals());
            }

            Some(OrganismAction::TryClone(child_size, child_minerals, direction)) => {
                if let Some(child) =
                    bot.split_off(child_size, child_minerals, self.config.mutation_chance)
                {
                    if let Some(WorldCell::Empty) = self.look_relative_mut((i, j), direction) {
                        let (new_i, new_j) = self.relative_shift((i, j), direction).unwrap();
                        self.field[new_i][new_j] = WorldCell::Organism(child);
                        self.field[i][j] = WorldCell::Organism(bot);
                    } else {
                        //bot dies if it cannot create a child when needed
                        self.field[i][j] =
                            WorldCell::DeadBody(bot.get_energy(), bot.get_minerals());
                    }
                }
            }

            Some(OrganismAction::ShareEnergy(amount, direction)) => {
                match self.look_relative_mut((i, j), direction) {
                    Some(WorldCell::DeadBody(ref mut n, _)) => *n += amount,
                    Some(WorldCell::Organism(ref mut o)) => o.add_energy(amount),
                    Some(c @ WorldCell::Empty) => *c = WorldCell::DeadBody(amount, 0),
                    None => {}
                }
                self.field[i][j] = WorldCell::Organism(bot);
            }

            Some(OrganismAction::ShareMinerals(amount, direction)) => {
                let max_minerals = self.config.max_minerals;
                match self.look_relative_mut((i, j), direction) {
                    Some(WorldCell::DeadBody(_, ref mut n)) => *n += amount,
                    Some(WorldCell::Organism(ref mut o)) => o.add_minerals(amount, max_minerals),
                    Some(c @ WorldCell::Empty) => *c = WorldCell::DeadBody(0, amount),
                    None => {}
                }
                self.field[i][j] = WorldCell::Organism(bot);
            }

            None => {
                let child = if let (Ok((child_size, child_minerals)), false) = (
                    (self.config.split_behaviour)(bot.get_energy(), bot.get_minerals()),
                    bot.can_clone,
                ) {
                    bot.split_off(child_size, child_minerals, self.config.mutation_chance)
                } else if bot.get_energy() > self.config.max_cell_size {
                    let (energy, minerals) =
                        (self.config.split_behaviour)(bot.get_energy(), bot.get_minerals())
                            .unwrap();
                    bot.split_off(energy, minerals, self.config.mutation_chance)
                } else {
                    None
                };

                if let Some(child) = child {
                    match self.try_place_bot((i, j), child) {
                        Ok(()) => {
                            self.field[i][j] = WorldCell::Organism(bot);
                        }

                        Err(()) => {
                            self.field[i][j] =
                                WorldCell::DeadBody(bot.get_energy(), bot.get_minerals());
                        }
                    }
                } else {
                    self.field[i][j] = WorldCell::Organism(bot);
                }
            }
        }
    }

    fn try_place_bot(&mut self, (i, j): (usize, usize), bot: Box<Organism>) -> Result<(), ()> {
        let mut directions = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        directions.shuffle(&mut thread_rng());
        for direction in directions {
            if let Some(WorldCell::Empty) = self.look_relative_mut((i, j), direction) {
                let (new_i, new_j) = self.relative_shift((i, j), direction).unwrap();
                self.field[new_i][new_j] = WorldCell::Organism(bot);
                return Ok(());
            }
        }
        Err(())
    }

    pub fn tick(&mut self) {
        for i in 0..self.get_height() {
            for j in 0..self.get_width() {
                if self.updates[i][j] == self.iteration {
                    continue;
                }

                let mut possible_bot = WorldCell::Empty;
                mem::swap(&mut self.field[i][j], &mut possible_bot);

                match possible_bot {
                    WorldCell::Organism(o) => {
                        self.process_bot((i, j), o);
                    }
                    b @ WorldCell::DeadBody(..) => {
                        self.field[i][j] = b;
                    }
                    _ => {}
                }

                self.updates[i][j] = self.updates[i][j].wrapping_add(1);
            }
        }

        self.iteration = self.iteration.wrapping_add(1);
        self.total_steps += BigUint::from(1usize);
    }
}

impl Display for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.get_height() {
            for j in 0..self.get_height() {
                write!(
                    f,
                    "{: <4}",
                    match &self.field[i][j] {
                        WorldCell::Empty => {
                            " ".to_string()
                        }
                        WorldCell::DeadBody(..) => {
                            "b".to_string()
                        }
                        WorldCell::Organism(o) => {
                            format!("{}", o.get_energy())
                        }
                    }
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
