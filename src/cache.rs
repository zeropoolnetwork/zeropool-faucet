use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, Instant},
};

// No need to use redis for caching. For now, shared HashMap is more than sufficient.
pub struct TtlCache<V> {
    map: HashMap<V, Instant>,
    ttl: Duration,
}

impl<V: Hash + Eq> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            map: HashMap::new(),
            ttl,
        }
    }

    pub fn add(&mut self, value: V) {
        self.cleanup();
        self.map.insert(value, Instant::now());
    }

    pub fn contains(&self, value: &V) -> bool {
        if let Some(instant) = self.map.get(value).cloned() {
            instant.elapsed() < self.ttl
        } else {
            false
        }
    }

    // pub fn get(&mut self, value: &V) -> Option<&Instant> {
    //     if let Some(instant) = self.map.get(value).cloned() {
    //         if instant.elapsed() < self.ttl {
    //             return Some(instant);
    //         } else {
    //             None;
    //         }
    //     }
    //
    //     None
    // }

    pub fn cleanup(&mut self) {
        self.map.retain(|_, instant| instant.elapsed() < self.ttl);
    }
}
