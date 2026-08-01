use std::collections::HashMap;
use std::hash::Hash;

use super::remote::Stale;

pub struct Cache<K, V> {
    entries: HashMap<K, V>,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Default> Cache<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn get_or_default(&mut self, key: &K) -> &mut V {
        self.entries.entry(key.clone()).or_default()
    }

    pub fn get_or_insert_with(&mut self, key: K, make: impl FnOnce() -> V) -> &mut V {
        self.entries.entry(key).or_insert_with(make)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, value);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.values_mut()
    }

    pub fn retain(&mut self, keep: impl FnMut(&K, &mut V) -> bool) {
        self.entries.retain(keep);
    }
}

impl<K: Eq + Hash + Clone, V: Default + Stale> Cache<K, V> {
    pub fn invalidate_all(&mut self) {
        for value in self.entries.values_mut() {
            value.mark_stale();
        }
    }
}
