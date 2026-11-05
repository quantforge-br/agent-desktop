use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::center::Center;
use super::fake_nodes::{Described, Note, Tree, card, element, node};
use super::fake_quirks::Quirks;
use super::fake_signals::DuxSignals;
use super::parse::parse;
use super::world::DuxWorld;
use crate::notifications::signal::Signal;
use crate::system::shell::dux::details::Details;
use crate::system::shell::search::walk;
use crate::tree::AtspiElement;
use crate::tree::vocabulary::role_numbers::{FILLER, FRAME, PUSH_BUTTON, RADIO_BUTTON};

struct State {
    open: bool,
    history: bool,
    notes: Vec<Note>,
    expanded: Vec<&'static str>,
    reads: usize,
    layout_reads: usize,
    torn_reads: usize,
}

/// The shell's laid-out state, as a model of the live dux-shell: a group of
/// several shows its newest card until it is expanded, a collapsed group's
/// dismiss removes the whole group, an action closes the center, and the
/// service reports a dismissal for each notification closed.
pub(super) struct FakeDux {
    state: RefCell<State>,
    pub(super) quirks: Cell<Quirks>,
    pub(super) log: Rc<RefCell<Vec<String>>>,
    pub(super) heard: Rc<RefCell<Vec<Signal>>>,
    presses: Cell<usize>,
}

pub(super) fn shell(notes: Vec<Note>, open: bool) -> FakeDux {
    FakeDux {
        state: RefCell::new(State {
            open,
            history: false,
            notes,
            expanded: Vec::new(),
            reads: 0,
            layout_reads: 0,
            torn_reads: 0,
        }),
        quirks: Cell::new(Quirks::default()),
        log: Rc::new(RefCell::new(Vec::new())),
        heard: Rc::new(RefCell::new(Vec::new())),
        presses: Cell::new(0),
    }
}

impl FakeDux {
    pub(super) fn quirks(&self, quirks: Quirks) -> &Self {
        self.quirks.set(quirks);
        self
    }

    /// Shows no notification cards until the center has been read `reads`
    /// times since it opened, as a center still laying out its list does.
    pub(super) fn lays_out_after(&self, reads: usize) -> &Self {
        self.state.borrow_mut().layout_reads = reads;
        self
    }

    /// Fails the next `reads` readings after a close, as a center tearing
    /// its tree down while it leaves does.
    pub(super) fn tears_on_close(&self, reads: usize) -> &Self {
        self.state.borrow_mut().torn_reads = reads;
        self
    }

    pub(super) fn titles(&self) -> Vec<&'static str> {
        self.state
            .borrow()
            .notes
            .iter()
            .map(|note| note.title)
            .collect()
    }

    pub(super) fn expand(&self, app: &'static str) {
        self.state.borrow_mut().expanded.push(app);
    }

    pub(super) fn close_now(&self) {
        self.state.borrow_mut().open = false;
    }

    pub(super) fn collapse(&self, app: &'static str) {
        self.state.borrow_mut().expanded.retain(|seen| *seen != app);
    }

    pub(super) fn is_open(&self) -> bool {
        self.state.borrow().open
    }

    pub(super) fn is_expanded(&self, app: &str) -> bool {
        self.state.borrow().expanded.contains(&app)
    }

    pub(super) fn selects_history(&self) {
        self.state.borrow_mut().history = true;
    }

    pub(super) fn on_history(&self) -> bool {
        self.state.borrow().history
    }

    pub(super) fn pressed(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("press "))
            .cloned()
            .collect()
    }

    pub(super) fn signals(&self) -> DuxSignals {
        DuxSignals {
            heard: self.heard.clone(),
            fails: false,
        }
    }

    fn apps(state: &State) -> Vec<&'static str> {
        let mut apps: Vec<&'static str> = Vec::new();
        for note in &state.notes {
            if !apps.contains(&note.app) {
                apps.push(note.app);
            }
        }
        apps
    }

    fn tree(&self) -> (Tree, BTreeMap<String, Details>) {
        let quirks = self.quirks.get();
        let state = self.state.borrow();
        let (mut tree, mut details) = (Tree::new(), BTreeMap::new());
        let shown = state.open && !state.history && state.reads > state.layout_reads;
        node(&mut tree, None, "/f", FRAME, "", state.open);
        let current = format!("Current ({})", state.notes.len());
        node(
            &mut tree,
            Some("/f"),
            "/tab0",
            RADIO_BUTTON,
            &current,
            state.open,
        );
        node(
            &mut tree,
            Some("/f"),
            "/tab1",
            RADIO_BUTTON,
            "History (9)",
            state.open,
        );
        for header in ["/dnd", "/settings", "/clear", "/close"] {
            node(
                &mut tree,
                Some("/f"),
                header,
                PUSH_BUTTON,
                "Header",
                state.open,
            );
        }
        node(&mut tree, Some("/f"), "/filter", FILLER, "", state.open);
        details.insert(
            "/tab0".to_owned(),
            Details {
                description: String::new(),
                checked: !state.history,
            },
        );
        details.insert(
            "/tab1".to_owned(),
            Details {
                description: String::new(),
                checked: state.history,
            },
        );
        for app in Self::apps(&state) {
            let members: Vec<&Note> = state.notes.iter().filter(|note| note.app == app).collect();
            let expanded = state.expanded.contains(&app);
            let wrapper = format!("/g-{app}");
            node(&mut tree, Some("/f"), &wrapper, FILLER, "", shown);
            let lead = format!("{wrapper}/lead");
            card(
                &mut tree,
                (&wrapper, &lead),
                members[0],
                shown && !expanded,
                members.len() > 1,
            );
            details.insert(
                lead.clone(),
                Details {
                    description: members[0].body.to_owned(),
                    checked: false,
                },
            );
            node(
                &mut tree,
                Some(&wrapper),
                &format!("{wrapper}/dismiss"),
                PUSH_BUTTON,
                "Dismiss",
                shown && !expanded,
            );
            node(
                &mut tree,
                Some(&wrapper),
                &format!("{wrapper}/collapse"),
                PUSH_BUTTON,
                "Collapse",
                shown && expanded,
            );
            if expanded {
                let limit = quirks.expand_limit.unwrap_or(usize::MAX);
                for member in members.iter().take(limit) {
                    let path = format!("/n{}", member.id);
                    card(&mut tree, (&wrapper, &path), member, shown, false);
                    details.insert(
                        path,
                        Details {
                            description: member.body.to_owned(),
                            checked: false,
                        },
                    );
                }
            }
            node(
                &mut tree,
                Some(&wrapper),
                &format!("{wrapper}/menu"),
                FILLER,
                "",
                shown,
            );
        }
        (tree, details)
    }

    fn remove(&self, ids: &[u32]) {
        let mut state = self.state.borrow_mut();
        state.notes.retain(|note| !ids.contains(&note.id));
        if !self.quirks.get().silent_service {
            for id in ids {
                self.heard.borrow_mut().push(Signal::Closed(*id, 2));
            }
        }
    }

    fn press_card(&self, path: &str) {
        let notes = self.state.borrow().notes.clone();
        let Some((card, control)) = path.rsplit_once('.') else {
            return;
        };
        let id = card
            .strip_prefix("/n")
            .and_then(|id| id.parse::<u32>().ok());
        if let Some(found) = notes.iter().find(|note| Some(note.id) == id) {
            self.press_note(found, control, &notes);
        }
    }

    fn press_note(&self, note: &Note, control: &str, notes: &[Note]) {
        let quirks = self.quirks.get();
        if control == "dismiss" && !quirks.inert_dismiss {
            let mut ids = vec![note.id];
            if quirks.dismiss_takes_neighbour {
                ids.extend(
                    notes
                        .iter()
                        .find(|other| other.id != note.id)
                        .map(|other| other.id),
                );
            }
            self.remove(&ids);
        }
        if control.starts_with("act") {
            self.heard.borrow_mut().push(Signal::ActionInvoked(note.id));
            self.remove(&[note.id]);
            if !quirks.keeps_open_after_action {
                self.state.borrow_mut().open = false;
            }
        }
    }

    fn press_group(&self, path: &str) {
        let (group, control) = path.rsplit_once('/').unwrap_or((path, ""));
        let apps = Self::apps(&self.state.borrow());
        let named = group.strip_prefix("/g-");
        let Some(app) = apps.into_iter().find(|app| Some(*app) == named) else {
            return;
        };
        let members: Vec<Note> = self
            .state
            .borrow()
            .notes
            .iter()
            .filter(|note| note.app == app)
            .cloned()
            .collect();
        let ids: Vec<u32> = members.iter().map(|note| note.id).collect();
        match control {
            "collapse" => self.state.borrow_mut().expanded.retain(|seen| *seen != app),
            "dismiss" | "lead.dismiss" if !self.quirks.get().inert_dismiss => {
                let mut gone = ids.clone();
                if self.quirks.get().dismiss_takes_neighbour && control == "lead.dismiss" {
                    let all = self.state.borrow().notes.clone();
                    gone.extend(
                        all.iter()
                            .find(|other| other.app != app)
                            .map(|other| other.id),
                    );
                }
                self.remove(&gone);
            }
            "lead.toggle" => self.state.borrow_mut().expanded.push(app),
            lead if lead.starts_with("lead.act") => {
                self.press_note(&members[0], &lead["lead.".len()..], &members);
            }
            _ => {}
        }
    }
}

impl DuxWorld for FakeDux {
    fn center(&self, deadline: Deadline) -> Result<Option<Center>, AdapterError> {
        if self.quirks.get().unreadable_after_press && self.presses.get() > 0 {
            return Err(AdapterError::new(ErrorCode::Internal, "the bus went away"));
        }
        let closed = self.log.borrow().iter().any(|line| line == "ipc close");
        let mut state = self.state.borrow_mut();
        if closed && state.torn_reads > 0 {
            state.torn_reads -= 1;
            return Err(AdapterError::new(
                ErrorCode::Internal,
                "the tree changed under the read",
            ));
        }
        if !state.open {
            return Ok(None);
        }
        state.reads += 1;
        drop(state);
        let (tree, details) = self.tree();
        let read = Described(tree);
        let walked = walk(&read, &element("/f"), deadline, &|_| false)?;
        let center = parse(&walked, &|found| {
            Ok(details.get(found.path()).cloned().unwrap_or_default())
        })?;
        Ok(center.is_open().then_some(center))
    }

    fn open(&self, _: Deadline) -> Result<String, AdapterError> {
        self.log.borrow_mut().push("ipc open".to_owned());
        if !self.quirks.get().never_shows {
            self.state.borrow_mut().open = true;
            self.state.borrow_mut().reads = 0;
        }
        Ok("NOTIFICATION_MODAL_OPEN_SUCCESS".to_owned())
    }

    fn close(&self, _: Deadline) -> Result<(), AdapterError> {
        self.log.borrow_mut().push("ipc close".to_owned());
        self.state.borrow_mut().open = false;
        Ok(())
    }

    fn press(&self, node: &AtspiElement, _: Deadline) -> Result<(), AdapterError> {
        let path = node.path().to_owned();
        self.log.borrow_mut().push(format!("press {path}"));
        self.presses.set(self.presses.get() + 1);
        match path.as_str() {
            "/tab0" => self.state.borrow_mut().history = false,
            "/tab1" => self.state.borrow_mut().history = true,
            _ if path.starts_with("/g-") => self.press_group(&path),
            _ => self.press_card(&path),
        }
        Ok(())
    }
}
