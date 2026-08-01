use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use shakmaty::zobrist::Zobrist64;
use crate::model::{Evaluation, EngineMetadata};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub fen: String,
    pub evaluation: Evaluation,
    pub metadata: EngineMetadata,
}

pub struct PositionCache {
    entries: HashMap<u64, CacheEntry>,
}

impl PositionCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn lookup(&self, hash: Zobrist64, fen: &str, requested_depth: u32) -> Option<(Evaluation, EngineMetadata)> {
        // Zobrist64 in shakmaty implements Zobrist hash, let's extract the underlying u64 value.
        // It has a .0 or we can convert it or format it, but let's check if we can get u64 by treating it as u64.
        // Let's use Zobrist64's underlying u64.
        let key = hash.0;
        if let Some(entry) = self.entries.get(&key) {
            if normalize_fen(&entry.fen) == normalize_fen(fen) {
                if entry.metadata.depth >= requested_depth {
                    return Some((entry.evaluation.clone(), entry.metadata.clone()));
                }
            }
        }
        None
    }

    pub fn insert(&mut self, hash: Zobrist64, fen: String, evaluation: Evaluation, metadata: EngineMetadata) {
        let key = hash.0;
        if let Some(existing) = self.entries.get(&key) {
            if normalize_fen(&existing.fen) == normalize_fen(&fen) {
                if existing.metadata.depth >= metadata.depth {
                    return; // Existing is deeper or equal, do not overwrite
                }
            }
        }
        self.entries.insert(key, CacheEntry {
            fen,
            evaluation,
            metadata,
        });
    }
}

pub fn normalize_fen(fen: &str) -> String {
    fen.split_whitespace().take(4).collect::<Vec<&str>>().join(" ")
}

#[derive(Clone)]
pub struct SharedPositionCache {
    inner: Arc<RwLock<PositionCache>>,
}

impl SharedPositionCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PositionCache::new())),
        }
    }

    pub fn lookup(&self, hash: Zobrist64, fen: &str, requested_depth: u32) -> Option<(Evaluation, EngineMetadata)> {
        let cache = self.inner.read().unwrap();
        cache.lookup(hash, fen, requested_depth)
    }

    pub fn insert(&self, hash: Zobrist64, fen: String, evaluation: Evaluation, metadata: EngineMetadata) {
        let mut cache = self.inner.write().unwrap();
        cache.insert(hash, fen, evaluation, metadata);
    }
}
