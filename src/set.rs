use crate::map::PatriciaTreeMap;

#[derive(Debug)]
pub struct PatriciaTreeSet {
    base: PatriciaTreeMap<()>,
}

impl PatriciaTreeSet {
    pub fn new() -> Self {
        Self {
            base: PatriciaTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.base.len()
    }

    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    pub fn contains(&self, key: u64) -> bool {
        self.base.contains(key)
    }

    pub fn insert(&mut self, key: u64) -> bool {
        self.base.insert(key, ()).is_none()
    }
}

impl Default for PatriciaTreeSet {
    fn default() -> Self {
        Self::new()
    }
}
