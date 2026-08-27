use std::{
    hash::{DefaultHasher, Hash, Hasher},
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use lru::LruCache;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::AppError;

const SHARD_COUNT: usize = 16;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct AlertSignature {
    pub node_id: Uuid,
    pub rule_identifier: String,
    pub mitre_technique: Option<String>,
}

pub struct ShardedDeduplicator {
    shards: Vec<Mutex<LruCache<AlertSignature, (Instant, u64)>>>,
    suppression_window: Duration,
}

impl ShardedDeduplicator {
    pub fn new(total_capacity: usize, suppression_window: Duration) -> Result<Self, AppError> {
        let per_shard = total_capacity
            .checked_div(SHARD_COUNT)
            .and_then(NonZeroUsize::new)
            .ok_or_else(|| AppError::Config(format!("dedup capacity must be >= SHARD_COUNT ({SHARD_COUNT})")))?;

        let shards = (0..SHARD_COUNT).map(|_| Mutex::new(LruCache::new(per_shard))).collect();

        Ok(Self {
            shards,
            suppression_window,
        })
    }

    fn shard_index(&self, sig: &AlertSignature) -> usize {
        let mut hasher = DefaultHasher::new();
        sig.node_id.hash(&mut hasher);
        (hasher.finish() as usize) % SHARD_COUNT
    }

    pub async fn check_and_record(&self, sig: &AlertSignature) -> bool {
        let idx = self.shard_index(sig);
        let mut lock = self.shards[idx].lock().await;
        let now = Instant::now();

        if let Some((last_seen, count)) = lock.get_mut(sig) {
            if now.duration_since(*last_seen) < self.suppression_window {
                *count += 1;
                return false;
            }
            *last_seen = now;
            *count = 1;
            true
        } else {
            lock.put(sig.clone(), (now, 1));
            true
        }
    }
}
