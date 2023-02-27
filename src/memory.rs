// TODO: document members of the module where it makes sense

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::helpers::pulls::Conflict;
use crate::structs::*;

/// The two-level pull request storage (repository -> pull number -> pull object)
#[derive(Default, Debug, Clone)]
pub struct Memory {
    pub pulls: Arc<Mutex<HashMap<String, HashMap<i32, PullRequest>>>>,
    pub conflicts: Arc<Mutex<HashMap<String, HashMap<i32, Vec<Conflict>>>>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_conflicts(&self, full_repo_name: &str, pull_number: i32, conflicts: Vec<Conflict>) {
        self.conflicts
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
            .entry(pull_number)
            .or_default()
            .extend(conflicts)
    }

    pub fn insert_pull(&self, full_repo_name: &str, new_pull: PullRequest) {
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

    pub fn remove_pull(&self, full_repo_name: &str, p: &PullRequest) {
        if let Some(pulls) = self.pulls.lock().unwrap().get_mut(full_repo_name) {
            pulls.remove(&p.number);
        }
        if let Some(conflicts) = self.conflicts.lock().unwrap().get_mut(full_repo_name) {
            conflicts.remove(&p.number);
        }
    }

    pub fn pulls(&self, full_repo_name: &str) -> Option<HashMap<i32, PullRequest>> {
        self.pulls
            .lock()
            .unwrap()
            .get(&full_repo_name.to_string())
            .map(|m| m.clone())
    }

    pub fn conflicts(&self, full_repo_name: &str) -> HashMap<i32, Vec<Conflict>> {
        match self
            .conflicts
            .lock()
            .unwrap()
            .get(&full_repo_name.to_string())
        {
            Some(cc) => cc.clone(),
            None => HashMap::new(),
        }
    }

    pub fn replace_conflicts(&self, full_repo_name: &str, conflicts: HashMap<i32, Vec<Conflict>>) {
        self.conflicts.lock().unwrap().insert(full_repo_name.to_string(), conflicts);
    }   

    pub fn drop_repository(&self, full_repo_name: &str) {
        self.pulls
            .lock()
            .unwrap()
            .remove(&full_repo_name.to_string());
        self.conflicts
            .lock()
            .unwrap()
            .remove(&full_repo_name.to_string());
    }
}
