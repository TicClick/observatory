use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CGroup {
    pub path: PathBuf,
}

impl CGroup {
    pub fn current() -> Self {
        let pid = std::process::id();
        if let Ok(cgroup_name) = std::fs::read_to_string(format!("/proc/{pid}/cgroup")) {
            if let Some(mut cgroup_path) = cgroup_name.strip_prefix("0::/") {
                cgroup_path = cgroup_path.trim();
                if !cgroup_path.is_empty() {
                    return CGroup {
                        path: Path::new(&format!("/sys/fs/cgroup/{cgroup_path}")).to_path_buf(),
                    };
                }
            }
        }
        Self {
            path: PathBuf::new(),
        }
    }

    pub fn valid(&self) -> bool {
        self.path.exists()
    }

    fn read(&self, subsystem: &str) -> Option<String> {
        std::fs::read_to_string(self.path.as_path().join(subsystem)).ok()
    }

    pub fn summary(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        if !self.valid() {
            return m;
        }
        for subsystem in [
            "memory.current",
            "memory.pressure",
            "cpu.pressure",
            "io.pressure",
            "cgroup.threads",
        ] {
            if let Some(data) = self.read(subsystem) {
                m.insert(subsystem.to_owned(), data);
            }
        }
        m
    }
}
