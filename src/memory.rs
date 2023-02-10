// TODO: document members of the module where it makes sense

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::structs::*;

#[derive(Default, Debug, Clone)]
pub struct Memory {
    pub pulls: Arc<Mutex<HashMap<i32, PullRequest>>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, new_pull: PullRequest) {
        let mut g = self.pulls.lock().unwrap();
        if let Some(pull) = g.get(&new_pull.number) {
            if pull.updated_at >= new_pull.updated_at {
                return;
            }
        }
        g.insert(new_pull.number, new_pull);
    }

    pub fn remove(&self, p: &PullRequest) {
        self.pulls.lock().unwrap().remove(&p.number);
    }
}
