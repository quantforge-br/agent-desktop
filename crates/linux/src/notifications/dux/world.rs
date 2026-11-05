use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::center::Center;
use super::parse::parse;
use crate::system::SharedBus;
use crate::system::shell::dux::app::find_app;
use crate::system::shell::dux::backend::DuxShell;
use crate::system::shell::dux::details::{LiveReader, Reader};
use crate::system::shell::dux::header::tab_header_start;
use crate::system::shell::dux::ipc::{Ipc, QuickshellIpc};
use crate::system::shell::light::{Light, LiveNodes, Nodes};
use crate::system::shell::search::walk;
use crate::tree::AtspiElement;

const IPC_TARGET: &str = "notifications";
const TORN_RETRIES: usize = 8;
const TORN_GAP: Duration = Duration::from_millis(80);

/// What the notification code needs from the dux-shell, so its tests run on a
/// fixed shell.
pub(crate) trait DuxWorld {
    /// The center when it is on screen, `None` while it is closed.
    fn center(&self, deadline: Deadline) -> Result<Option<Center>, AdapterError>;
    /// Asks the shell to show the center and answers with what the shell said.
    fn open(&self, deadline: Deadline) -> Result<String, AdapterError>;
    fn close(&self, deadline: Deadline) -> Result<(), AdapterError>;
    fn press(&self, node: &AtspiElement, deadline: Deadline) -> Result<(), AdapterError>;
}

pub(crate) struct LiveDuxWorld<'a> {
    bus: &'a SharedBus,
    shell: DuxShell,
    ipc: QuickshellIpc,
}

impl<'a> LiveDuxWorld<'a> {
    pub(crate) fn new(bus: &'a SharedBus, shell: DuxShell) -> Self {
        Self {
            bus,
            shell,
            ipc: QuickshellIpc::of(shell.pid),
        }
    }
}

fn ambiguous() -> AdapterError {
    AdapterError::ambiguous_target("Several windows of the shell look like the notification center")
}

fn torn() -> AdapterError {
    AdapterError::new(
        ErrorCode::Internal,
        "The notification center kept changing while it was read",
    )
    .with_suggestion("Retry the command")
}

fn candidates<N: Nodes>(nodes: &N, app: &AtspiElement) -> Result<Vec<Light>, AdapterError> {
    let frames = nodes.read_level(&nodes.read(app)?.children);
    let mut found = Vec::new();
    for frame in frames {
        let frame = match frame {
            Ok(frame) => frame,
            Err(error) if error.code == ErrorCode::ElementNotFound => continue,
            Err(error) => return Err(error),
        };
        let kids = nodes.read_level(&frame.children);
        let roles: Vec<u32> = kids
            .iter()
            .filter_map(|kid| kid.as_ref().ok().map(|light| light.role))
            .collect();
        if tab_header_start(&roles).is_some() {
            found.push(frame);
        }
    }
    Ok(found)
}

/// One pass over the shell's windows: the open center, or a sign that a node
/// vanished while the tree was walked and the pass has to be repeated.
enum Pass {
    Read(Box<Option<Center>>),
    Torn,
}

impl LiveDuxWorld<'_> {
    fn pass(&self, deadline: Deadline) -> Result<Pass, AdapterError> {
        let app = find_app(self.bus, self.shell.pid, deadline)?;
        let nodes = LiveNodes::new(self.bus.get(deadline)?.inner().clone(), deadline);
        let reader = LiveReader::new(self.bus, deadline)?;
        let mut open = Vec::new();
        for frame in candidates(&nodes, &app)? {
            let walked = walk(&nodes, &frame.element, deadline, &|_| false)?;
            if !walked.complete {
                return Ok(Pass::Torn);
            }
            let center = parse(&walked, &|element| reader.details(element))?;
            if center.is_open() {
                open.push(center);
            }
        }
        match open.len() {
            0 => Ok(Pass::Read(Box::new(None))),
            1 => Ok(Pass::Read(Box::new(open.pop()))),
            _ => Err(ambiguous()),
        }
    }

    fn require_handler(&self, deadline: Deadline) -> Result<(), AdapterError> {
        if self.ipc.exposes(IPC_TARGET, "open", deadline)? {
            return Ok(());
        }
        Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "This dux-shell build offers no notification center IPC handler",
        )
        .with_suggestion("Open the notification center by hand and retry")
        .with_details(json!({ "ipc": "handler_missing", "target": IPC_TARGET }))
        .with_disposition(DeliverySemantics::not_delivered()))
    }
}

impl DuxWorld for LiveDuxWorld<'_> {
    fn center(&self, deadline: Deadline) -> Result<Option<Center>, AdapterError> {
        for _ in 0..TORN_RETRIES {
            if let Pass::Read(center) = self.pass(deadline)? {
                return Ok(*center);
            }
            std::thread::sleep(TORN_GAP);
        }
        Err(torn())
    }

    fn open(&self, deadline: Deadline) -> Result<String, AdapterError> {
        self.require_handler(deadline)?;
        self.ipc.call(IPC_TARGET, "open", deadline)
    }

    fn close(&self, deadline: Deadline) -> Result<(), AdapterError> {
        self.ipc.call(IPC_TARGET, "close", deadline).map(|_| ())
    }

    fn press(&self, node: &AtspiElement, deadline: Deadline) -> Result<(), AdapterError> {
        LiveReader::new(self.bus, deadline)?.press(node)
    }
}
