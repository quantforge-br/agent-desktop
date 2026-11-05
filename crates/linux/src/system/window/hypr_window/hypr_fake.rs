use std::cell::{Cell, RefCell};

use agent_desktop_core::{AdapterError, Deadline, ProcessId, Rect, WindowInfo, WindowState};

use super::hypr_world::HyprWorld;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_client_owner::HyprClientOwner;
use crate::system::hyprland::hypr_client_state::HyprClientState;
use crate::system::hyprland::hypr_command::HyprCommand;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::process::process_facts::ProcessFacts;
use crate::system::process::process_lookup::Lookup;

pub(crate) const PID: u32 = 4242;
pub(crate) const INSTANCE: &str = "boot:1";

/// A compositor that applies dispatchers the way the measured Hyprland
/// does: a resize keeps the window's centre, `ignored` commands are accepted
/// and change nothing, and focus follows the focus dispatcher.
pub(crate) struct FakeHypr {
    pub(crate) clients: RefCell<Vec<HyprClient>>,
    pub(crate) monitors: Vec<HyprMonitor>,
    pub(crate) active: Cell<Option<u64>>,
    pub(crate) sent: RefCell<Vec<HyprCommand>>,
    pub(crate) ignored: Vec<HyprCommand>,
    pub(crate) deadline: Deadline,
    pub(crate) instance: &'static str,
}

pub(crate) fn client(address: u64, title: &str, frame: Rect) -> HyprClient {
    HyprClient {
        address,
        stable_id: format!("{address:x}"),
        owner: HyprClientOwner {
            pid: PID,
            class: "app".into(),
            title: title.into(),
            xwayland: false,
        },
        frame,
        state: HyprClientState {
            workspace: "1".into(),
            monitor: 0,
            floating: false,
            fullscreen: 0,
            mapped: true,
            hidden: false,
            pinned: false,
        },
        focus_history: 0,
    }
}

pub(crate) fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn window(title: &str) -> WindowInfo {
    WindowInfo {
        id: "w-1".into(),
        title: title.into(),
        app: "app".into(),
        pid: ProcessId::new(PID),
        process_instance: Some(INSTANCE.into()),
        bounds: None,
        state: WindowState::default(),
    }
}

impl FakeHypr {
    pub(crate) fn with(clients: Vec<HyprClient>) -> Self {
        Self {
            clients: RefCell::new(clients),
            monitors: vec![HyprMonitor {
                id: 0,
                name: "DP-1".into(),
                logical: rect(0.0, 0.0, 1920.0, 1080.0),
                scale: 1.0,
                focused: true,
                active_workspace: "3".into(),
                special_workspace: None,
            }],
            active: Cell::new(None),
            sent: RefCell::new(Vec::new()),
            ignored: Vec::new(),
            deadline: Deadline::detached_after(2_000).unwrap(),
            instance: INSTANCE,
        }
    }

    pub(crate) fn only(&self) -> HyprClient {
        self.clients.borrow()[0].clone()
    }

    fn apply(&self, client: &mut HyprClient, command: &HyprCommand) {
        let frame = &mut client.frame;
        match command {
            HyprCommand::Float(on) => client.state.floating = *on,
            HyprCommand::MoveTo { x, y } => {
                frame.x = *x as f64;
                frame.y = *y as f64;
            }
            HyprCommand::ResizeTo { width, height } => {
                frame.x += (frame.width - *width as f64) / 2.0;
                frame.y += (frame.height - *height as f64) / 2.0;
                frame.width = *width as f64;
                frame.height = *height as f64;
            }
            HyprCommand::Maximize(on) => client.state.fullscreen = u8::from(*on),
            HyprCommand::Fullscreen(on) => client.state.fullscreen = if *on { 2 } else { 0 },
            HyprCommand::ToWorkspace(workspace) => {
                client.state.workspace = workspace.clone();
                if let Some(monitor) = self.monitors.iter().find(|m| m.shows(workspace)) {
                    client.state.monitor = monitor.id;
                }
            }
            HyprCommand::Focus | HyprCommand::Close => {}
        }
    }
}

impl HyprWorld for FakeHypr {
    fn clients(&self) -> Result<Vec<HyprClient>, AdapterError> {
        Ok(self.clients.borrow().clone())
    }

    fn monitors(&self) -> Result<Vec<HyprMonitor>, AdapterError> {
        Ok(self.monitors.clone())
    }

    fn active(&self) -> Result<Option<u64>, AdapterError> {
        Ok(self.active.get())
    }

    fn send(&self, address: u64, command: &HyprCommand) -> Result<(), AdapterError> {
        self.sent.borrow_mut().push(command.clone());
        if self.ignored.contains(command) {
            return Ok(());
        }
        if *command == HyprCommand::Focus {
            self.active.set(Some(address));
        }
        let mut clients = self.clients.borrow_mut();
        if let Some(client) = clients.iter_mut().find(|client| client.address == address) {
            self.apply(client, command);
        }
        Ok(())
    }

    fn lookup(&self, _pid: u32) -> Lookup {
        Lookup::Found(ProcessFacts {
            executable: None,
            comm: "app".into(),
            exe_basename: None,
            instance: self.instance.into(),
        })
    }

    fn stopped(&self, _pid: u32) -> bool {
        false
    }

    fn deadline(&self) -> Deadline {
        self.deadline
    }
}
