use std::{collections::HashMap, sync::Arc};

use arc_swap::{ArcSwap, Guard};

use crate::mitre::MitreTaxonomy;

#[derive(Clone)]
pub struct EngineRegistry {
    pub rule_sets: HashMap<String, Arc<yara_x::Rules>>,
    pub mitre: MitreTaxonomy,
}

impl EngineRegistry {
    pub fn new(rule_sets: HashMap<String, yara_x::Rules>, mitre: MitreTaxonomy) -> Self {
        let rule_sets = rule_sets.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();
        Self { rule_sets, mitre }
    }
}

pub struct RegistryHolder {
    inner: ArcSwap<EngineRegistry>,
}

impl RegistryHolder {
    pub fn new(registry: EngineRegistry) -> Self {
        Self {
            inner: ArcSwap::from_pointee(registry),
        }
    }

    pub fn load(&self) -> Guard<Arc<EngineRegistry>> {
        self.inner.load()
    }

    /// Atomically swaps the active registry in memory.
    /// In-flight evaluations holding an active Guard continue evaluating against the previous ruleset
    /// without blocking or corrupting memory state.
    pub fn swap(&self, new_registry: EngineRegistry) {
        self.inner.store(Arc::new(new_registry));
    }
}
