use agent_desktop_core::{AdapterError, Deadline};

use super::app::find_app;
use super::backend::DuxShell;
use super::frame::Frame;
use super::ipc::{Ipc, QuickshellIpc};
use super::layer::Layer;
use super::layers;
use super::present::Stage;
use crate::system::SharedBus;
use crate::system::shell::light::{Light, LiveNodes, Nodes};

fn roles_of(nodes: &LiveNodes, frame: &Light) -> Vec<u32> {
    nodes
        .read_level(&frame.children)
        .into_iter()
        .filter_map(|kid| kid.ok().map(|light| light.role))
        .collect()
}

/// The running dux-shell.
pub(crate) struct LiveStage<'a> {
    bus: &'a SharedBus,
    shell: DuxShell,
    ipc: QuickshellIpc,
}

impl<'a> LiveStage<'a> {
    pub(crate) fn new(bus: &'a SharedBus, shell: DuxShell) -> Self {
        Self {
            bus,
            shell,
            ipc: QuickshellIpc::of(shell.pid),
        }
    }
}

impl Stage for LiveStage<'_> {
    fn pid(&self) -> u32 {
        self.shell.pid
    }

    fn layers(&self, deadline: Deadline) -> Result<Vec<Layer>, AdapterError> {
        layers::read(deadline)
    }

    fn frames(&self, deadline: Deadline) -> Result<(String, Vec<Frame>), AdapterError> {
        let app = find_app(self.bus, self.shell.pid, deadline)?;
        let nodes = LiveNodes::new(self.bus.get(deadline)?.inner().clone(), deadline);
        let root = nodes.read(&app)?;
        let frames = nodes
            .read_level(&root.children)
            .into_iter()
            .filter_map(Result::ok)
            .map(|light| Frame {
                width: light.extents.map_or(0, |extents| i64::from(extents.width)),
                height: light.extents.map_or(0, |extents| i64::from(extents.height)),
                showing: light.showing,
                children: light.children.len(),
                roles: roles_of(&nodes, &light),
                element: light.element,
            })
            .collect();
        Ok((root.name, frames))
    }

    fn ipc(&self) -> &dyn Ipc {
        &self.ipc
    }
}
