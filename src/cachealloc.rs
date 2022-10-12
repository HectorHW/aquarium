use serde::Deserialize;

use crate::cells::organism::Organism;

#[derive(Clone, Debug, Default)]
pub struct ObjectCache<const N: usize> {
    internal_buffer: heapless::Vec<Box<Organism>, N>,
}

impl<const N: usize> ObjectCache<N> {
    pub fn new() -> Self {
        ObjectCache {
            internal_buffer: heapless::Vec::new(),
        }
    }

    pub fn store_drop(&mut self, item: Box<Organism>) -> bool {
        self.internal_buffer.push(item).is_ok()
    }

    pub fn get_alloc(&mut self) -> Box<Organism> {
        self.internal_buffer
            .pop()
            .unwrap_or_else(|| Box::new(Default::default()))
    }
}

impl<'de, const N: usize> Deserialize<'de> for ObjectCache<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Default::default())
    }
}
