/// No need to use redis for caching. For now, shared HashMap is more than sufficient.
use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, Instant},
};
use std::{fmt::Display, net::IpAddr, ops::SubAssign};

pub trait Balance: Eq + SubAssign + Ord + Clone + Copy + Display {}
impl<B> Balance for B where B: Eq + SubAssign + Ord + Clone + Copy + Display {}

pub struct AddrCache<B> {
    pub addresses: Cache<String, B>,
    pub ips: Cache<IpAddr, B>,
}

impl<B: Balance> AddrCache<B> {
    pub fn new(reset_interval: Duration, max_value: B) -> Self {
        Self {
            addresses: Cache::new(reset_interval, max_value),
            ips: Cache::new(reset_interval, max_value),
        }
    }

    pub fn can_spend(&self, to: &str, ip: IpAddr, amount: B) -> bool {
        let can_spend_ip = self.ips.can_spend(ip.to_owned(), amount);
        let can_spend_addr = self.addresses.can_spend(to.to_owned(), amount);

        can_spend_addr && can_spend_ip
    }

    pub fn spend(&mut self, to: String, ip: IpAddr, amount: B) {
        self.ips.spend(ip, amount);
        self.addresses.spend(to, amount);
    }
}

struct CacheEntry<B> {
    remaining_value: B,
    last_update: Instant,
}

impl<B> CacheEntry<B> {
    fn new(remaining_value: B) -> Self {
        Self {
            remaining_value,
            last_update: Instant::now(),
        }
    }

    fn is_expired(&self, interval: Duration) -> bool {
        self.last_update.elapsed() > interval
    }
}

pub struct Cache<K, B> {
    max_value: B,
    map: HashMap<K, CacheEntry<B>>,
    ttl: Duration,
}

impl<K: Hash + Eq + Display + Clone, B: Balance> Cache<K, B> {
    pub fn new(ttl: Duration, max_value: B) -> Self {
        Self {
            max_value,
            map: HashMap::new(),
            ttl,
        }
    }

    pub fn spend(&mut self, key: K, amount: B) -> bool {
        self.cleanup();

        let entry = self
            .map
            .entry(key.clone())
            .or_insert_with(|| CacheEntry::new(self.max_value));

        if entry.remaining_value >= amount {
            tracing::info!("Spending {} for {}", amount, key);
            entry.remaining_value -= amount;
            true
        } else {
            false
        }
    }

    pub fn can_spend(&self, key: K, amount: B) -> bool {
        self.map
            .get(&key)
            .map(|entry| entry.is_expired(self.ttl) || entry.remaining_value >= amount)
            .unwrap_or(true)
    }

    fn cleanup(&mut self) {
        self.map.retain(|_, entry| !entry.is_expired(self.ttl));
    }
}
