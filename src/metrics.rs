use std::collections::HashMap;

use anyhow::Result;
use sysinfo::{Pid, Process, System};

pub struct Metrics {
    pub sys: System,
}

impl Metrics {
    pub fn init() -> Result<Self> {
        let mut sys = System::new_all();
        sys.refresh_all();
        std::thread::sleep(std::time::Duration::from_millis(300));
        sys.refresh_all();
        Ok(Self { sys })
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
    }

    pub fn global_cpu(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    pub fn memory(&self) -> (u64, u64) {
        (self.sys.used_memory(), self.sys.total_memory())
    }

    pub fn processes(&self) -> &HashMap<Pid, Process> {
        self.sys.processes()
    }
}
