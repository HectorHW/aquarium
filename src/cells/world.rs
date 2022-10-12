use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use rand::{distributions::Bernoulli, prelude::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};

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
pub enum WorldCellInner {
    Empty,
    Organism(Organism),
    DeadBody(usize, usize),
}

impl WorldCellInner {
    fn unwrap_bot(&self) -> Option<&Organism> {
        match self {
            WorldCellInner::Organism(o) => Some(o),
            _ => None,
        }
    }

    fn unwrap_bot_mut(&mut self) -> Option<&mut Organism> {
        match self {
            WorldCellInner::Organism(o) => Some(o),
            _ => None,
        }
    }
}

pub type WorldCell = Box<WorldCellInner>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldField {
    pub inner: Vec<WorldCell>,
    width: usize,
}

impl Index<(usize, usize)> for WorldField {
    type Output = WorldCellInner;

    fn index(&self, (i, j): (usize, usize)) -> &Self::Output {
        self.inner[i * self.width + j].as_ref()
    }
}

impl IndexMut<(usize, usize)> for WorldField {
    fn index_mut(&mut self, (i, j): (usize, usize)) -> &mut Self::Output {
        self.inner[i * self.width + j].as_mut()
    }
}

impl WorldField {
    pub fn get(&self, (i, j): (usize, usize)) -> Option<&WorldCellInner> {
        let pos = i * self.width + j;
        self.inner.get(pos).map(|i| i.as_ref())
    }

    pub fn get_mut(&mut self, (i, j): (usize, usize)) -> Option<&mut WorldCellInner> {
        let pos = i * self.width + j;
        self.inner.get_mut(pos).map(|i| i.as_mut())
    }

    pub fn get_mut_box(&mut self, (i, j): (usize, usize)) -> Option<&mut WorldCell> {
        let pos = i * self.width + j;
        self.inner.get_mut(pos)
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

    /// perform directional look. Returns reference to current cell and maybe ref to cell we are looking at
    fn look_relative_disjoint_mut(
        &mut self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<(&mut WorldCellInner, Option<&mut WorldCellInner>)> {
        match self.relative_shift((i, j), direction) {
            Some(p2) => {
                let refs = self.disjoint_refs((i, j), p2)?;
                Some((refs.0, Some(refs.1)))
            }
            None => {
                return self.get_mut((i, j)).map(|item| (item, None));
            }
        }
    }
    /// perform directional look. returns references to corresponding boxes, if any
    fn look_relative_disjoint_mut_boxes(
        &mut self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<(&mut WorldCell, Option<&mut WorldCell>)> {
        match self.relative_shift((i, j), direction) {
            Some(p2) => {
                let refs = self.disjoint_refs((i, j), p2)?;
                Some((refs.0, Some(refs.1)))
            }
            None => {
                return self.get_mut_box((i, j)).map(|item| (item, None));
            }
        }
    }

    pub fn look_relative_mut(
        &mut self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<&mut WorldCellInner> {
        let (i, j) = self.relative_shift((i, j), direction)?;
        Some(&mut self[(i, j)])
    }

    pub fn look_relative(
        &self,
        (i, j): (usize, usize),
        direction: Direction,
    ) -> Option<&WorldCellInner> {
        let (i, j) = self.relative_shift((i, j), direction)?;
        Some(&self[(i, j)])
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.inner.len() / self.width
    }

    pub fn disjoint_refs(
        &mut self,
        (i1, j1): (usize, usize),
        (i2, j2): (usize, usize),
    ) -> Option<(&mut WorldCell, &mut WorldCell)> {
        if i1 == i2 && j1 == j2 {
            return None;
        }

        let slice = &mut self.inner[..];

        let first = (&mut slice[i1 * self.width + j1]) as *mut WorldCell;
        let second = (&mut slice[i2 * self.width + j2]) as *mut WorldCell;
        unsafe { Some((&mut *first, &mut *second)) }
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
        let field = vec![Box::new(WorldCellInner::Empty); WIDTH * HEIGHT];
        World {
            field: WorldField {
                width: WIDTH,
                inner: field,
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
                if matches!(self.field[(i, j)], WorldCellInner::Empty) {
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

            self.field[(i, j)] = WorldCellInner::Organism(organism);
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

    pub fn get_light(config: &WorldConfig, i: usize) -> usize {
        (config.light_behaviour)(i)
    }

    pub fn get_minerals(config: &WorldConfig, i: usize) -> usize {
        (config.minerals_behaviour)(i)
    }

    #[inline(always)]
    fn run_bot_prelude(config: &WorldConfig, (i, _j): (usize, usize), bot: &mut Organism) {
        let minerals = Self::get_minerals(config, i);
        bot.add_minerals(minerals, config.max_minerals);
        bot.age(&config.aging_mutation_freq);
    }

    fn run_bot_action(&mut self, (i, j): (usize, usize)) -> ((usize, usize), Result<(), ()>) {
        let dead_energy = self.config.dead_energy;
        let attack_cost = self.config.attack_cost;

        /*
        safety: world is passed inside as immutable, so it is ok for bot to look at it.
        bot may change itself

        */

        let const_world = unsafe { &*(self as *const World) };

        let action = {
            self.field[(i, j)]
                .unwrap_bot_mut()
                .unwrap()
                .tick(const_world, (i, j))
        };

        match action {
            Some(OrganismAction::TryEat(direction)) => {
                let other_index = self
                    .field
                    .look_relative_disjoint_mut((i, j), direction)
                    .unwrap();

                let (bot, other_cell) = (other_index.0.unwrap_bot_mut().unwrap(), other_index.1);

                match other_cell {
                    Some(WorldCellInner::Organism(ref mut other))
                        if bot.get_energy() > attack_cost =>
                    {
                        let energy = other.get_energy();

                        let chance = mass_to_chance(bot.get_energy(), energy);
                        bot.decrease_energy(attack_cost);
                        if chance {
                            bot.add_energy(energy.saturating_sub(dead_energy) / 2);
                            *self.field.look_relative_mut((i, j), direction).unwrap() =
                                WorldCellInner::Empty;
                        } else {
                            other.register_attack(direction.inverse());
                        }
                    }

                    Some(cell @ &mut WorldCellInner::DeadBody(..)) => {
                        let (energy, minerals) = match &cell {
                            WorldCellInner::DeadBody(e, m) => (*e, *m),
                            _ => unreachable!(),
                        };
                        *cell = WorldCellInner::Empty;
                        bot.add_energy(energy / 2);
                        bot.add_minerals(minerals / 2, self.config.max_minerals);
                    }

                    _any_other_case => {}
                }
            }
            Some(OrganismAction::TryMove(direction)) => {
                let (this, other) = self
                    .field
                    .look_relative_disjoint_mut_boxes((i, j), direction)
                    .unwrap();

                if let Some(WorldCellInner::Empty) =
                    other.as_ref().map(|inner| inner.as_ref()).as_ref()
                {
                    //we expect moves to happen often, so we will make it cheap by swapping pointers
                    std::mem::swap(this, other.unwrap());

                    let new_pos = self.field.relative_shift((i, j), direction).unwrap();
                    //dont forget to tick new cell instead
                    return (new_pos, Ok(()));
                }
            }

            Some(OrganismAction::Die) => {
                return ((i, j), Err(()));
            }

            Some(OrganismAction::TryClone(child_size, child_minerals, direction)) => {
                if let (WorldCellInner::Organism(bot), Some(child_pos @ WorldCellInner::Empty)) =
                    self.field
                        .look_relative_disjoint_mut((i, j), direction)
                        .unwrap()
                {
                    bot.try_clone_into(
                        child_pos,
                        child_size,
                        child_minerals,
                        self.config.mutation_chance,
                    );
                }
            }

            Some(OrganismAction::ShareEnergy(amount, direction)) => {
                if let Some(WorldCellInner::Organism(ref mut o)) =
                    self.field.look_relative_mut((i, j), direction)
                {
                    o.add_energy(amount)
                }
            }

            Some(OrganismAction::ShareMinerals(amount, direction)) => {
                let max_minerals = self.config.max_minerals;
                if let Some(WorldCellInner::Organism(ref mut o)) =
                    self.field.look_relative_mut((i, j), direction)
                {
                    o.add_minerals(amount, max_minerals)
                }
            }

            None => {}
        }
        ((i, j), Ok(()))
    }

    #[inline(always)]
    fn run_bot_postlude(config: &WorldConfig, (_i, _j): (usize, usize), bot: &mut Organism) {
        // 1 is already subtracted via action
        bot.decrease_energy(energy_soft_cap(bot.get_energy(), config.max_cell_size));
    }

    fn mark_tick(&mut self, pos: (usize, usize)) {
        *self.get_update_mut(pos) = self.iteration;
    }

    #[inline(always)]
    fn process_bot(&mut self, (mut i, mut j): (usize, usize)) -> (usize, usize) {
        Self::run_bot_prelude(
            &self.config,
            (i, j),
            self.field[(i, j)].unwrap_bot_mut().unwrap(),
        );

        let (tick_idx, result) = self.run_bot_action((i, j));

        (i, j) = tick_idx;

        self.mark_tick(tick_idx);

        match result {
            Ok(_) => {}
            Err(_) => {
                self.field[(i, j)] = WorldCellInner::DeadBody(
                    self.config.dead_energy,
                    self.field
                        .get(tick_idx)
                        .unwrap()
                        .unwrap_bot()
                        .unwrap()
                        .get_minerals(),
                );
                return (i, j);
            }
        }

        let (bot, target) = self
            .field
            .look_relative_disjoint_mut(tick_idx, Direction::get_random())
            .unwrap();
        let bot = bot.unwrap_bot_mut().unwrap();

        if let (Ok((child_size, child_minerals)), false) = (
            (self.config.split_behaviour)(bot.get_energy(), bot.get_minerals()),
            bot.can_clone,
        ) {
            if let Some(target @ WorldCellInner::Empty) = target {
                bot.try_clone_into(
                    target,
                    child_size,
                    child_minerals,
                    self.config.mutation_chance,
                );
            }
        }

        Self::run_bot_postlude(&self.config, (i, j), bot);
        (i, j)
    }

    pub fn tick(&mut self) {
        for mut i in 0..self.get_height() {
            for mut j in 0..self.get_width() {
                if self.get_update((i, j)) == self.iteration {
                    continue;
                }

                if let WorldCellInner::Organism(_) = &self.field[(i, j)] {
                    (i, j) = self.process_bot((i, j));
                }

                *self.get_update_mut((i, j)) = self.iteration;
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
                        WorldCellInner::Empty => {
                            " ".to_string()
                        }
                        WorldCellInner::DeadBody(..) => {
                            "b".to_string()
                        }
                        WorldCellInner::Organism(o) => {
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
