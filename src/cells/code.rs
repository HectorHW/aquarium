use std::fmt::Display;
use std::ops::Deref;

use itertools::Itertools;
use rand::distributions::{Bernoulli, Standard};
use rand::prelude::Distribution;
use rand::{thread_rng, Rng};

use serde::{Deserialize, Serialize};

pub const CODE_SIZE: usize = 256;

use serde_big_array::big_array;

big_array! { BigOpcodeArray;}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    #[serde(with = "BigOpcodeArray")]
    pub code: [OpCode; CODE_SIZE],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    Clone,
    Compare,

    UseMinerals,
    Share,
    ShareMinerals,
    Sythesize,
}

impl OpCode {
    fn display_postioned(&self, idx: usize, codesize: usize) -> String {
        format!(
            "{: <4} {}",
            idx,
            match self {
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
                    format!("jump {shift} (to {})", (idx + *shift as usize) % codesize)
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
                OpCode::Clone => "clone".to_string(),
                OpCode::Sythesize => "photosynthesize".to_string(),
                OpCode::Compare => "compare".to_string(),
                OpCode::UseMinerals => "use minerals".to_string(),
                OpCode::Share => "share energy".to_string(),
                OpCode::ShareMinerals => "share minerals".to_string(),
            }
        )
    }
}

impl Distribution<OpCode> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OpCode {
        use OpCode::*;
        let param: u8 = rng.gen();
        match rng.gen_range(0..=16) {
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
            11 => Clone,
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
        Program {
            code: items.into_array().unwrap(),
        }
    }

    ///probability is counted as mutation_chance/1000
    pub fn clone_lossy(&self, mutation_chance: usize) -> Self {
        let mut items = heapless::Vec::<OpCode, CODE_SIZE>::new();
        for idx in 0..CODE_SIZE {
            if thread_rng().gen::<usize>() % 1000usize < mutation_chance {
                items.push(rand::random()).unwrap();
            } else {
                items.push(self.code[idx]).unwrap();
            }
        }
        Program {
            code: items.into_array().unwrap(),
        }
    }

    pub fn break_with_chance(&mut self, damage_chance: &Bernoulli) {
        if damage_chance.sample(&mut thread_rng()) {
            let instruction = &mut self.code[thread_rng().gen::<usize>() % self.code.len()];
            *instruction = thread_rng().gen();
        }
    }

    pub fn print_minimized(&self, mut ip: usize) -> String {
        let mut markers = vec![false; 256];
        let mut resume_point = vec![0];

        while !markers[ip] || !resume_point.is_empty() {
            if markers[ip] {
                ip = resume_point.pop().unwrap();
                continue;
            }
            markers[ip] = true;
            match self.code[ip] {
                OpCode::JumpUnconditional(offset) => {
                    ip = (ip + offset as usize) % self.code.len();
                }
                OpCode::SkipZero(_) => {
                    resume_point.push((ip + 1) % self.code.len());
                    ip = (ip + 2) % self.code.len();
                }
                _ => {
                    ip = (ip + 1) % self.code.len();
                }
            }
        }

        self.code
            .iter()
            .enumerate()
            .filter_map(|(idx, instr)| {
                if !markers[idx] {
                    return None;
                }
                Some(instr.display_postioned(idx, self.code.len()))
            })
            .join("\n")
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.code
                .iter()
                .enumerate()
                .map(|(idx, instr)| { instr.display_postioned(idx, self.code.len()) })
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl Deref for Program {
    type Target = [OpCode; CODE_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.code
    }
}

#[cfg(test)]
mod test {
    use super::Program;

    #[test]
    fn test_program_serialization() {
        let program = Program::random_program();

        let json = serde_json::to_string(&program).unwrap();

        let recovered: Program = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(program, recovered);
    }
}
