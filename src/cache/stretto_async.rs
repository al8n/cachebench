use std::sync::Arc;

use async_trait::async_trait;

use super::{AsyncCacheDriver, Counters, DefaultHasher, Key, Value};
use crate::{config::Config, parser::TraceEntry, report::Report, EvictionCounters};
use stretto::TokioRuntime;

#[derive(Clone)]
pub struct StrettoAsyncCache {
    config: Arc<Config>,
    #[allow(clippy::type_complexity)]
    cache: stretto::AsyncCache<
        Key,
        Value,
        stretto::DefaultKeyBuilder<Key>,
        stretto::DefaultCoster<Value>,
        stretto::DefaultUpdateValidator<Value>,
        stretto::DefaultCacheCallback<Value>,
        DefaultHasher,
    >,
}

impl StrettoAsyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        if let Some(_ttl) = config.ttl {
            todo!()
        }
        if let Some(_tti) = config.tti {
            todo!()
        }
        if config.size_aware {
            todo!()
        }

        Self {
            config: Arc::new(config.clone()),
            cache: stretto::AsyncCache::builder(capacity * 10, capacity as i64)
                .set_hasher(DefaultHasher)
                .finalize::<TokioRuntime>()
                .unwrap(),
        }
    }

    async fn get(&self, key: &usize) -> bool {
        self.cache.get(key).await.is_some()
    }

    async fn insert(&self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_task_for_insertion(&self.config).await;
        self.cache.insert(key, value, 1).await;
    }
}

#[async_trait]
impl AsyncCacheDriver<TraceEntry> for StrettoAsyncCache {
    async fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            if self.get(&block).await {
                counters.read_hit();
            } else {
                self.insert(block, req_id).await;
                counters.inserted();
                counters.read_missed();
            }
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    async fn get_or_insert_once(&mut self, _entry: &TraceEntry, _report: &mut Report) {
        unimplemented!();
    }

    async fn update(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            self.insert(block, req_id).await;
            counters.inserted();
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    async fn invalidate(&mut self, _entry: &TraceEntry) {
        unimplemented!();
    }

    fn invalidate_all(&mut self) {
        unimplemented!();
    }

    fn invalidate_entries_if(&mut self, _entry: &TraceEntry) {
        unimplemented!();
    }

    async fn iterate(&mut self) {
        unimplemented!();
    }

    fn eviction_counters(&self) -> Option<Arc<EvictionCounters>> {
        None
    }
}
