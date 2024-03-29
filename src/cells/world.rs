use std::{
    fmt::Display,
    mem,
    ops::{Index, IndexMut},
};

use rand::{distributions::Bernoulli, prelude::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::cachealloc::ObjectCache;

use super::organism::{Direction, Organism, OrganismAction};

#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub start_energy: usize,
    pub dead_energy: usize,
    pub split_behaviour: fn(usize, usize) -> Result<(usize, usize), ()>,
    pub light_behaviour: fn(usize) -> usize,
    pub minerals_behaviour: fn(usize) -> usize,
    pub mutation_chance: usize,
    pub aging_mutation_freq: Bernoulli,
    pub max_cell_size: usize,
    pub max_minerals: usize,
    pub attack_cost: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldCell {
    Empty,
    Organism(Box<Organism>),
    DeadBody(usize, usize),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldField {
    pub inner: Vec<WorldCell>,

    #[serde(skip_serializing)]
    #[serde(default)]
    cache: ObjectCache<2500>,
    width: usize,
}

impl Index<(usize, usize)> for WorldField {
    type Output = WorldCell;

    fn index(&self, (i, j): (usize, usize)) -> &Self::Output {
        &self.inner[i * self.width + j]
    }
}

impl IndexMut<(usize, usize)> for WorldField {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut Self::Output {
        &mut self.inner[i * self.width + j]
    }
}

impl WorldField {
    pub fn get(&self, (i, j): (usize, usize)) -> Option<&WorldCell> {
        let pos = i * self.width + j;
        self.inner.get(pos)
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.inner.len() / self.width
    }
}

pub struct World {
    pub field: WorldField,
    iteration: usize,
    updates: Vec<usize>,
    width: usize,

    pub config: WorldConfig,
    pub measure_steps: usize,
}

impl World {
    pub fn empty<const WIDTH: usize, const HEIGHT: usize>(config: WorldConfig) -> Self {
        let field = vec![WorldCell::Empty; WIDTH * HEIGHT];
        World {
            field: WorldField {
                width: WIDTH,
                inner: field,
                cache: Default::default(),
            },
            iteration: 1,
            updates: vec![0; WIDTH * HEIGHT],
            width: WIDTH,
            config,
            measure_steps: 0usize,
        }
    }

    fn get_free_cells(&self) -> Vec<(usize, usize)> {
        let mut res = vec![];
        for j in 0..self.field.width {
            for i in 0..self.field.inner.len() / self.field.width {
                if matches!(self.field[(i, j)], WorldCell::Empty) {
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

            let mut allocation = self.field.cache.get_alloc();
            *allocation.as_mut() = bot_factory(self.config.start_energy);

            self.field[(i, j)] = WorldCell::Organism(allocation);
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
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.field.inner.len() / self.width
    }

    fn get_update(&self, (i, j): (usize, usize)) -> usize {
        self.updates[i * self.width + j]
    }

    fn get_update_mut(&mut self, (i, j): (usize, usize)) -> &mut usize {
        &mut self.updates[i * self.width + j]
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
        Some(&mut self.field[(i, j)])
    }

    pub fn look_relative(
        &self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<&WorldCell> {
        let (i, j) = self.relative_shift((i, j), direction)?;
        Some(&self.field[(i, j)])
    }

    pub fn get_light(&self, i: usize) -> usize {
        (self.config.light_behaviour)(i)
    }

    pub fn get_minerals(&self, i: usize) -> usize {
        (self.config.minerals_behaviour)(i)
    }

    #[inline(always)]
    fn run_bot_prelude(&mut self, (i, _j): (usize, usize), bot: &mut Organism) {
        let minerals = self.get_minerals(i);
        bot.add_minerals(minerals, self.config.max_minerals);
        bot.age(&self.config.aging_mutation_freq);
    }

    fn run_bot_action(
        &mut self,
        (i, j): (&mut usize, &mut usize),
        bot: &mut Organism,
    ) -> Result<(), ()> {
        match bot.tick(self, (*i, *j)) {
            Some(OrganismAction::TryEat(direction)) => {
                let dead_energy = self.config.dead_energy;
                let attack_cost = self.config.attack_cost;
                match self.look_relative_mut((*i, *j), direction) {
                    Some(&mut WorldCell::Organism(ref mut other))
                        if bot.get_energy() > attack_cost =>
                    {
                        let energy = other.get_energy();

                        let chance = mass_to_chance(bot.get_energy(), energy);
                        bot.decrease_energy(attack_cost);
                        if chance {
                            bot.add_energy(energy.saturating_sub(dead_energy) / 2);
                            *self.look_relative_mut((*i, *j), direction).unwrap() =
                                WorldCell::Empty;
                        } else {
                            other.register_attack(direction.inverse());
                        }
                    }

                    Some(cell @ &mut WorldCell::DeadBody(..)) => {
                        let (energy, minerals) = match &cell {
                            WorldCell::DeadBody(e, m) => (*e, *m),
                            _ => unreachable!(),
                        };
                        *cell = WorldCell::Empty;
                        bot.add_energy(energy / 2);
                        bot.add_minerals(minerals / 2, self.config.max_minerals);
                    }

                    _any_other_case => {}
                }
            }
            Some(OrganismAction::TryMove(direction)) => {
                if let Some(WorldCell::Empty) = self.look_relative_mut((*i, *j), direction) {
                    let (new_i, new_j) = self.relative_shift((*i, *j), direction).unwrap();
                    *i = new_i;
                    *j = new_j;
                    *self.get_update_mut((*i, *j)) = self.get_update((*i, *j)).wrapping_add(1);
                }
            }

            Some(OrganismAction::Die) => {
                return Err(());
            }

            Some(OrganismAction::TryClone(child_size, child_minerals, direction)) => {
                if let Some(WorldCell::Empty) = self.look_relative_mut((*i, *j), direction) {
                    if let Some(child) = bot.split_off(
                        || self.field.cache.get_alloc(),
                        child_size,
                        child_minerals,
                        self.config.mutation_chance,
                    ) {
                        let pos = self.relative_shift((*i, *j), direction).unwrap();
                        self.field[pos] = WorldCell::Organism(child);
                        return Ok(());
                    }
                }
            }

            Some(OrganismAction::ShareEnergy(amount, direction)) => {
                if let Some(WorldCell::Organism(ref mut o)) =
                    self.look_relative_mut((*i, *j), direction)
                {
                    o.add_energy(amount)
                }
            }

            Some(OrganismAction::ShareMinerals(amount, direction)) => {
                let max_minerals = self.config.max_minerals;
                if let Some(WorldCell::Organism(ref mut o)) =
                    self.look_relative_mut((*i, *j), direction)
                {
                    o.add_minerals(amount, max_minerals)
                }
            }

            None => {}
        }
        Ok(())
    }

    #[inline(always)]
    fn run_bot_postlude(&mut self, (_i, _j): (usize, usize), bot: &mut Organism) {
        // 1 is already subtracted via action
        bot.decrease_energy(energy_soft_cap(bot.get_energy(), self.config.max_cell_size));
    }

    #[inline(always)]
    fn process_bot(&mut self, (mut i, mut j): (usize, usize), mut bot: Box<Organism>) {
        self.run_bot_prelude((i, j), bot.as_mut());

        match self.run_bot_action((&mut i, &mut j), bot.as_mut()) {
            Ok(_) => {}
            Err(_) => {
                self.field[(i, j)] =
                    WorldCell::DeadBody(self.config.dead_energy, bot.get_minerals());

                self.field.cache.store_drop(bot);

                return;
            }
        }

        let child = if let (Ok((child_size, child_minerals)), false) = (
            (self.config.split_behaviour)(bot.get_energy(), bot.get_minerals()),
            bot.can_clone,
        ) {
            bot.split_off(
                || self.field.cache.get_alloc(),
                child_size,
                child_minerals,
                self.config.mutation_chance,
            )
        } else {
            None
        };

        if let Some(child) = child {
            if let Err(bot) = self.try_place_bot((i, j), child) {
                self.field.cache.store_drop(bot);
            }
        }

        self.run_bot_postlude((i, j), bot.as_mut());

        self.field[(i, j)] = WorldCell::Organism(bot);
    }

    #[inline]
    fn try_place_bot(
        &mut self,
        (i, j): (usize, usize),
        bot: Box<Organism>,
    ) -> Result<(), Box<Organism>> {
        let mut directions = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        directions.shuffle(&mut thread_rng());
        for direction in directions {
            if let Some(WorldCell::Empty) = self.look_relative_mut((i, j), direction) {
                let pos = self.relative_shift((i, j), direction).unwrap();
                self.field[pos] = WorldCell::Organism(bot);
                return Ok(());
            }
        }
        Err(bot)
    }

    pub fn tick(&mut self) {
        for i in 0..self.get_height() {
            for j in 0..self.get_width() {
                if self.get_update((i, j)) == self.iteration {
                    continue;
                }

                let mut possible_bot = WorldCell::Empty;
                mem::swap(&mut self.field[(i, j)], &mut possible_bot);

                match possible_bot {
                    WorldCell::Organism(o) => {
                        self.process_bot((i, j), o);
                    }
                    b @ WorldCell::DeadBody(..) => {
                        self.field[(i, j)] = b;
                    }
                    _ => {}
                }

                *self.get_update_mut((i, j)) = self.get_update((i, j)).wrapping_add(1);
            }
        }

        self.iteration = self.iteration.wrapping_add(1);
        self.measure_steps += 1;
    }
}

impl Display for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.get_height() {
            for j in 0..self.get_height() {
                write!(
                    f,
                    "{: <4}",
                    match &self.field[(i, j)] {
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

/// computate chance of eating based on masses of two cells
#[inline(always)]
fn mass_to_chance(own_mass: usize, target_mass: usize) -> bool {
    thread_rng().gen_ratio(own_mass as u32, (own_mass + target_mass + 1) as u32)
}

#[inline(always)]
fn energy_soft_cap(mass: usize, cap: usize) -> usize {
    (mass as f64 / cap as f64).ceil() as usize
}
