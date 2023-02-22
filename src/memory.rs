// TODO: document members of the module where it makes sense

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::structs::*;

/// The two-level pull request storage (repository -> pull number -> pull object)
#[derive(Default, Debug, Clone)]
pub struct Memory {
    pub pulls: Arc<Mutex<HashMap<String, HashMap<i32, PullRequest>>>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, full_repo_name: &str, new_pull: PullRequest) {
        let mut g = self.pulls.lock().unwrap();
        if let Some(pull) = g
            .entry(full_repo_name.to_string())
            .or_default()
            .get(&new_pull.number)
        {
            if pull.updated_at >= new_pull.updated_at {
                return;
            }
        }
        g.entry(full_repo_name.to_string())
            .or_default()
            .insert(new_pull.number, new_pull);
    }

    pub fn remove(&self, full_repo_name: &str, p: &PullRequest) {
        if let Some(pulls) = self.pulls.lock().unwrap().get_mut(full_repo_name) {
            pulls.remove(&p.number);
        }
    }

    pub fn drop_repository(&self, full_repo_name: &str) {
        self.pulls
            .lock()
            .unwrap()
            .remove(&full_repo_name.to_string());
    }
}
