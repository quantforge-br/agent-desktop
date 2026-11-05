use agent_desktop_core::{AdapterError, Deadline};

use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_command::HyprCommand;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::process::proc_stat;
use crate::system::process::process_facts;
use crate::system::process::process_lookup::Lookup;

/// What a Hyprland window operation reads and sends, so its decisions can
/// be driven by a fixed compositor.
pub(crate) trait HyprWorld {
    fn clients(&self) -> Result<Vec<HyprClient>, AdapterError>;
    fn monitors(&self) -> Result<Vec<HyprMonitor>, AdapterError>;
    fn active(&self) -> Result<Option<u64>, AdapterError>;
    fn send(&self, address: u64, command: &HyprCommand) -> Result<(), AdapterError>;
    fn lookup(&self, pid: u32) -> Lookup;
    fn stopped(&self, pid: u32) -> bool;
    fn deadline(&self) -> Deadline;
}

/// The running compositor, bounded by one command deadline.
pub(crate) struct LiveHypr {
    pub(crate) instance: HyprInstance,
    pub(crate) deadline: Deadline,
}

impl HyprWorld for LiveHypr {
    fn clients(&self) -> Result<Vec<HyprClient>, AdapterError> {
        self.instance.clients(self.deadline)
    }

    fn monitors(&self) -> Result<Vec<HyprMonitor>, AdapterError> {
        self.instance.monitors(self.deadline)
    }

    fn active(&self) -> Result<Option<u64>, AdapterError> {
        self.instance.active_address(self.deadline)
    }

    fn send(&self, address: u64, command: &HyprCommand) -> Result<(), AdapterError> {
        self.instance.dispatch(address, command, self.deadline)
    }

    fn lookup(&self, pid: u32) -> Lookup {
        process_facts::lookup(pid)
    }

    fn stopped(&self, pid: u32) -> bool {
        proc_stat::is_stopped(pid)
    }

    fn deadline(&self) -> Deadline {
        self.deadline
    }
}
