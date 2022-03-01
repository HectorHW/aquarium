use std::fmt::Display;

use rand::distributions::{Bernoulli, Standard};
use rand::prelude::Distribution;
use rand::{thread_rng, Rng};

pub const CODE_SIZE: usize = 256;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Program(pub [OpCode; CODE_SIZE]);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PackedAdressPair(u8);

impl PackedAdressPair {
    pub fn unwrap(&self) -> (usize, usize) {
        (self.0 as usize / 16, self.0 as usize % 16)
    }
}

impl From<u8> for PackedAdressPair {
    fn from(value: u8) -> Self {
        PackedAdressPair(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedAddress(u8);

impl PackedAddress {
    pub fn unwrap(&self) -> usize {
        self.0 as usize % 16
    }
}

impl From<u8> for PackedAddress {
    fn from(value: u8) -> Self {
        PackedAddress(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpCode {
    LoadInt(u8),
    CopyRegisters(PackedAdressPair),
    Add(PackedAdressPair),
    AddClip(PackedAdressPair),
    SubClip(PackedAdressPair),
    Flip(PackedAddress),
    JumpUnconditional(u8),
    SkipZero(PackedAddress),

    MoveRelative,
    LookRelative,

    Eat,
    Clone(u8),
    Compare,

    UseMinerals,
    Share,
    ShareMinerals,
    Sythesize,
}

impl Distribution<OpCode> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OpCode {
        use OpCode::*;
        let param: u8 = rng.gen();
        match rng.gen_range(0..=17) {
            0 => LoadInt(param),
            1 => CopyRegisters(param.into()),
            2 => Add(param.into()),
            3 => AddClip(param.into()),
            4 => SubClip(param.into()),
            5 => Flip(param.into()),
            6 => JumpUnconditional(param),
            7 => SkipZero(param.into()),
            8 => MoveRelative,
            9 => LookRelative,

            10 => OpCode::Eat,
            11 => Clone(param),
            12 => Compare,
            13 => UseMinerals,
            14 => Share,
            15 => ShareMinerals,

            _ => OpCode::Sythesize,
        }
    }
}

impl Program {
    pub fn random_program() -> Self {
        let mut items = heapless::Vec::<OpCode, CODE_SIZE>::new();
        for _ in 0..CODE_SIZE {
            items.push(rand::random()).unwrap();
        }
        Program(items.into_array().unwrap())
    }

    ///probability is counted as mutation_chance/1000
    pub fn clone_lossy(&self, mutation_chance: usize) -> Self {
        let mut items = heapless::Vec::<OpCode, CODE_SIZE>::new();
        for idx in 0..CODE_SIZE {
            if thread_rng().gen::<usize>() % 1000usize < mutation_chance {
                items.push(rand::random()).unwrap();
            } else {
                items.push(self.0[idx]).unwrap();
            }
        }
        Program(items.into_array().unwrap())
    }

    pub fn break_with_chance(&mut self, damage_chance: &Bernoulli) {
        if damage_chance.sample(&mut thread_rng()) {
            let instruction = &mut self.0[thread_rng().gen::<usize>() % self.0.len()];
            *instruction = thread_rng().gen();
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .enumerate()
                .map(|(idx, instr)| {
                    format!(
                        "{: <4} {}",
                        idx,
                        match instr {
                            OpCode::LoadInt(n) => format!("load int {}", n),
                            OpCode::CopyRegisters(addr) => {
                                let (from, to) = addr.unwrap();
                                format!("copy {to} <- {from}")
                            }
                            OpCode::Add(addr) => {
                                let (from, to) = addr.unwrap();
                                format!("add {to} <- {from}")
                            }
                            OpCode::AddClip(addr) => {
                                let (from, to) = addr.unwrap();
                                format!("add with clip {to} <- {from}")
                            }
                            OpCode::SubClip(addr) => {
                                let (from, to) = addr.unwrap();
                                format!("sub with clip {to} <- {from}")
                            }
                            OpCode::Flip(addr) => {
                                let from = addr.unwrap();
                                format!("flip register {from}")
                            }
                            OpCode::JumpUnconditional(shift) => {
                                format!(
                                    "jump {shift} (to {})",
                                    (idx + *shift as usize) % self.0.len()
                                )
                            }
                            OpCode::SkipZero(addr) => {
                                let addr = addr.unwrap();
                                format!("skip if register {addr} is 0")
                            }
                            OpCode::MoveRelative => {
                                "move relative".to_string()
                            }
                            OpCode::LookRelative => {
                                "look relative".to_string()
                            }
                            OpCode::Eat => "eat".to_string(),
                            OpCode::Clone(size) => {
                                let portion = *size as f64 / 256f64;
                                format!("clone giving {:.2}% mass", portion * 100f64)
                            }
                            OpCode::Sythesize => "photosynthesize".to_string(),
                            OpCode::Compare => "compare".to_string(),
                            OpCode::UseMinerals => "use minerals".to_string(),
                            OpCode::Share => "share energy".to_string(),
                            OpCode::ShareMinerals => "share minerals".to_string(),
                        }
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}
