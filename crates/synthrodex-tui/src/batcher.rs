#![allow(dead_code)]

use std::collections::VecDeque;

/// Batched X11 request for workspace/monitor state
#[derive(Debug, Clone)]
pub struct BatchRequest {
    pub request_type: BatchType,
    pub window: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BatchType {
    Workspace,
    Monitor,
}

/// State machine: Idle → Collecting → Flushing → Idle
#[derive(Debug, Clone, PartialEq)]
pub enum BatchState {
    Idle,
    Collecting,
    Flushing,
}

pub struct Batcher {
    queue: VecDeque<BatchRequest>,
    state: BatchState,
    window_count: usize,
}

impl Batcher {
    pub fn new(window_count: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            state: BatchState::Idle,
            window_count,
        }
    }

    pub fn queue_request(&mut self, request: BatchRequest) {
        self.queue.push_back(request);
        if self.state == BatchState::Idle {
            self.state = BatchState::Collecting;
        }
    }

    pub fn should_flush(&self) -> bool {
        self.state == BatchState::Collecting && self.queue.len() >= self.window_count
    }

    pub fn flush(&mut self) -> Vec<BatchRequest> {
        self.state = BatchState::Flushing;
        let mut batch = Vec::new();
        while let Some(req) = self.queue.pop_front() {
            batch.push(req);
        }
        self.state = BatchState::Idle;
        batch
    }

    pub fn state(&self) -> &BatchState {
        &self.state
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_idle() {
        let b = Batcher::new(3);
        assert_eq!(*b.state(), BatchState::Idle);
        assert_eq!(b.queue_len(), 0);
    }

    #[test]
    fn queue_request_transitions_to_collecting() {
        let mut b = Batcher::new(3);
        b.queue_request(BatchRequest {
            request_type: BatchType::Workspace,
            window: 1,
        });
        assert_eq!(*b.state(), BatchState::Collecting);
        assert_eq!(b.queue_len(), 1);
    }

    #[test]
    fn should_flush_when_full() {
        let mut b = Batcher::new(2);
        b.queue_request(BatchRequest {
            request_type: BatchType::Workspace,
            window: 1,
        });
        assert!(!b.should_flush());
        b.queue_request(BatchRequest {
            request_type: BatchType::Monitor,
            window: 2,
        });
        assert!(b.should_flush());
    }

    #[test]
    fn flush_drains_queue_and_returns_to_idle() {
        let mut b = Batcher::new(2);
        b.queue_request(BatchRequest {
            request_type: BatchType::Workspace,
            window: 1,
        });
        b.queue_request(BatchRequest {
            request_type: BatchType::Monitor,
            window: 2,
        });
        let batch = b.flush();
        assert_eq!(batch.len(), 2);
        assert_eq!(*b.state(), BatchState::Idle);
        assert_eq!(b.queue_len(), 0);
    }
}
