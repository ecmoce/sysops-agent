use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::StorageConfig;
use crate::types::{MetricId, MetricSample};

/// Thread-safe metric storage using ring buffers
#[derive(Clone)]
pub struct Storage {
    inner: Arc<RwLock<StorageInner>>,
}

struct StorageInner {
    buffers: HashMap<MetricId, RingBuffer>,
    capacity: usize,
}

struct RingBuffer {
    data: Vec<MetricSample>,
    head: usize,
    len: usize,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            head: 0,
            len: 0,
            capacity,
        }
    }

    fn push(&mut self, sample: MetricSample) {
        if self.data.len() < self.capacity {
            self.data.push(sample);
            self.len += 1;
        } else {
            self.data[self.head] = sample;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    fn latest(&self) -> Option<&MetricSample> {
        if self.data.is_empty() {
            return None;
        }
        let idx = if self.data.len() < self.capacity {
            self.data.len() - 1
        } else {
            (self.head + self.capacity - 1) % self.capacity
        };
        Some(&self.data[idx])
    }

    /// Get the most recent N samples in chronological order
    fn recent(&self, n: usize) -> Vec<&MetricSample> {
        let count = n.min(self.data.len());
        let mut result = Vec::with_capacity(count);

        if self.data.len() < self.capacity {
            // Buffer not yet full
            let start = self.data.len().saturating_sub(count);
            for i in start..self.data.len() {
                result.push(&self.data[i]);
            }
        } else {
            // Buffer is full, handle wrap-around
            let start = (self.head + self.capacity - count) % self.capacity;
            for i in 0..count {
                let idx = (start + i) % self.capacity;
                result.push(&self.data[idx]);
            }
        }

        result
    }
}

impl Storage {
    pub fn new(config: &StorageConfig) -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(StorageInner {
                buffers: HashMap::new(),
                capacity: config.ring_buffer_size as usize,
            })),
        })
    }

    pub fn insert(&self, sample: MetricSample) {
        let mut inner = self.inner.write().unwrap();
        let capacity = inner.capacity;
        let buffer = inner.buffers
            .entry(sample.metric)
            .or_insert_with(|| RingBuffer::new(capacity));
        buffer.push(sample);
    }

    pub fn latest(&self, metric: MetricId) -> Option<MetricSample> {
        let inner = self.inner.read().unwrap();
        inner.buffers.get(&metric)?.latest().cloned()
    }

    pub fn recent(&self, metric: MetricId, n: usize) -> Vec<MetricSample> {
        let inner = self.inner.read().unwrap();
        match inner.buffers.get(&metric) {
            Some(buf) => buf.recent(n).into_iter().cloned().collect(),
            None => Vec::new(),
        }
    }
}
