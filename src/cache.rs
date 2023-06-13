use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    net::IpAddr,
    time::{Duration, Instant},
};

// u128 is plenty for most blockchains and the intended amounts.
type Balance = u128;

pub struct AddrCache {
    pub addresses: Cache<String>,
    pub ips: Cache<IpAddr>,
}

impl AddrCache {
    pub fn new(reset_interval: Duration, max_value: Balance) -> Self {
        Self {
            addresses: Cache::new(reset_interval, max_value),
            ips: Cache::new(reset_interval, max_value),
        }
    }

    pub fn can_spend(&self, to: &str, ip: IpAddr, amount: Balance) -> bool {
        let can_spend_ip = self.ips.can_spend(&ip, amount);
        let can_spend_addr = self.addresses.can_spend(to, amount);

        can_spend_addr && can_spend_ip
    }

    pub fn spend(&mut self, to: &str, ip: IpAddr, amount: Balance) {
        self.ips.spend(ip, amount);
        self.addresses.spend(to.to_owned(), amount);
    }
}

struct CacheEntry {
    remaining_value: Balance,
    last_update: Instant,
}

impl CacheEntry {
    fn new(remaining_value: Balance) -> Self {
        Self {
            remaining_value,
            last_update: Instant::now(),
        }
    }

    fn is_expired(&self, interval: Duration) -> bool {
        self.last_update.elapsed() > interval
    }
}

pub struct Cache<K> {
    max_value: Balance,
    map: HashMap<K, CacheEntry>,
    ttl: Duration,
}

impl<K: Hash + Eq + Display + Clone> Cache<K> {
    pub fn new(ttl: Duration, max_value: Balance) -> Self {
        Self {
            max_value,
            map: HashMap::new(),
            ttl,
        }
    }

    pub fn spend(&mut self, key: K, amount: Balance) -> bool {
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

    pub fn can_spend<Q>(&self, key: &Q, amount: Balance) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map
            .get(key)
            .map(|entry| entry.is_expired(self.ttl) || entry.remaining_value >= amount)
            .unwrap_or(true)
    }

    fn cleanup(&mut self) {
        self.map.retain(|_, entry| !entry.is_expired(self.ttl));
    }
}
