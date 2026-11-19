use monitoring_proto::model::IngestPayload;
use std::collections::VecDeque;

/// Kapasitas maksimum paket yang ditahan di RAM saat jaringan putus (tanpa menulis ke SD card/disk)
pub const MAX_RING_BUFFER_PACKETS: usize = 50;

#[derive(Debug)]
pub struct InMemoryRingBuffer {
    queue: VecDeque<IngestPayload>,
    capacity: usize,
    dropped_count: u64,
}

impl Default for InMemoryRingBuffer {
    fn default() -> Self {
        Self::new(MAX_RING_BUFFER_PACKETS)
    }
}

impl InMemoryRingBuffer {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            queue: VecDeque::with_capacity(cap),
            capacity: cap,
            dropped_count: 0,
        }
    }

    /// Menambahkan paket ke antrean RAM. Jika penuh, buang paket paling lama (FIFO eviction).
    pub fn push(&mut self, mut payload: IngestPayload) {
        // Hemat RAM: jangan simpan duplikat raw Checkmk text di dalam ring buffer offline
        payload.checkmk_raw = None;
        if self.queue.len() >= self.capacity {
            self.queue.pop_front();
            self.dropped_count = self.dropped_count.saturating_add(1);
        }
        self.queue.push_back(payload);
    }

    pub fn peek_front(&self) -> Option<&IngestPayload> {
        self.queue.front()
    }

    pub fn pop_front(&mut self) -> Option<IngestPayload> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monitoring_proto::model::HostType;

    fn sample_payload(ts: i64) -> IngestPayload {
        IngestPayload {
            host_uuid: "uuid-1".to_string(),
            hostname: "HOST-1".to_string(),
            ip_address: "192.168.1.10".to_string(),
            mac_address: None,
            host_type: HostType::Linux,
            agent_version: "2.0.0".to_string(),
            timestamp_sec: ts,
            services: Vec::new(),
            inventory: None,
            checkmk_raw: Some("raw".to_string()),
        }
    }

    #[test]
    fn test_ring_buffer_evicts_oldest_when_exceeding_50() {
        let mut rb = InMemoryRingBuffer::new(50);
        for i in 1..=55 {
            rb.push(sample_payload(i));
        }
        assert_eq!(rb.len(), 50);
        assert_eq!(rb.dropped_count(), 5);
        assert_eq!(rb.pop_front().unwrap().timestamp_sec, 6);
    }
}
