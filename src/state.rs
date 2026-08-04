use std::collections::VecDeque;
use std::sync::Mutex;

use crate::render::RenderedPage;

pub struct Job {
    pub id: u64,
    pub when: String,
    pub peer: String,
    pub size: usize,
    pub raw: Vec<u8>,
    pub page: Option<RenderedPage>,
    pub summary: Vec<String>,
}

pub struct SharedState {
    pub jobs: Vec<Job>,
    pub logs: VecDeque<String>,
    pub total_bytes: u64,
    pub next_id: u64,
    pub paper_width: usize,
    pub server_error: Option<String>,
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            jobs: Vec::new(),
            logs: VecDeque::new(),
            total_bytes: 0,
            next_id: 1,
            paper_width: 576,
            server_error: None,
        }
    }

    pub fn log(&mut self, msg: String) {
        self.logs.push_back(format!("{} {}", now_hm(), msg));
        if self.logs.len() > 500 {
            self.logs.pop_front();
        }
    }
}

pub fn now_hm() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

pub type Shared = std::sync::Arc<Mutex<SharedState>>;
