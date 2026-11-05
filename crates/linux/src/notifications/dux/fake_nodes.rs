//! The node tree the fake dux-shell lays out, and the helpers its tests share.

use std::collections::BTreeMap;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use crate::system::shell::light::{Light, Nodes};
use crate::tree::AtspiElement;
use crate::tree::vocabulary::role_numbers::{LIST_ITEM, PUSH_BUTTON};

pub(super) fn element(path: &str) -> AtspiElement {
    AtspiElement::new(":1.2", path, 7)
}

pub(super) fn deadline() -> Deadline {
    Deadline::after(20_000).expect("a deadline")
}

pub(super) fn lease() -> agent_desktop_core::InteractionLease {
    agent_desktop_core::InteractionLease::guarded(deadline(), ()).expect("a lease")
}

/// One notification the fake shell holds.
#[derive(Clone)]
pub(super) struct Note {
    pub(super) id: u32,
    pub(super) app: &'static str,
    pub(super) title: &'static str,
    pub(super) body: &'static str,
    pub(super) actions: Vec<&'static str>,
}

pub(super) fn note(id: u32, app: &'static str, title: &'static str) -> Note {
    Note {
        id,
        app,
        title,
        body: "",
        actions: Vec::new(),
    }
}

pub(super) type Tree = BTreeMap<String, Light>;

pub(super) struct Described(pub(super) Tree);

impl Nodes for Described {
    fn read(&self, element: &AtspiElement) -> Result<Light, AdapterError> {
        self.0
            .get(element.path())
            .cloned()
            .ok_or_else(|| AdapterError::new(ErrorCode::ElementNotFound, "gone"))
    }
}

pub(super) fn node(
    tree: &mut Tree,
    parent: Option<&str>,
    path: &str,
    role: u32,
    name: &str,
    showing: bool,
) {
    tree.insert(
        path.to_owned(),
        Light {
            element: element(path),
            role,
            name: name.to_owned(),
            extents: None,
            showing,
            visible: showing,
            children: Vec::new(),
        },
    );
    if let Some(parent) = parent {
        if let Some(found) = tree.get_mut(parent) {
            found.children.push(element(path));
        }
    }
}

/// A notification card at `at`, a `(parent, path)` pair, with its controls.
pub(super) fn card(tree: &mut Tree, at: (&str, &str), note: &Note, showing: bool, toggle: bool) {
    let (parent, path) = at;
    let name = format!("{}. {}", note.app, note.title);
    node(tree, Some(parent), path, LIST_ITEM, &name, showing);
    node(
        tree,
        Some(path),
        &format!("{path}.toggle"),
        PUSH_BUTTON,
        "Expand",
        showing && toggle,
    );
    node(
        tree,
        Some(path),
        &format!("{path}.expand"),
        PUSH_BUTTON,
        "Expand",
        false,
    );
    node(
        tree,
        Some(path),
        &format!("{path}.close"),
        PUSH_BUTTON,
        "Dismiss",
        false,
    );
    for (at, action) in note.actions.iter().enumerate() {
        node(
            tree,
            Some(path),
            &format!("{path}.act{at}"),
            PUSH_BUTTON,
            action,
            showing,
        );
    }
    node(
        tree,
        Some(path),
        &format!("{path}.dismiss"),
        PUSH_BUTTON,
        "Dismiss",
        showing,
    );
}
