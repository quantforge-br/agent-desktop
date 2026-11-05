use agent_desktop_core::AdapterError;
use serde_json::{Value, json};

/// What became of the shell state a call changed, as read back afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Restore {
    Adopted,
    Closed,
    Unverified,
}

impl Restore {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::Closed => "closed",
            Self::Unverified => "unverified",
        }
    }

    pub(crate) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unverified, _) | (_, Self::Unverified) => Self::Unverified,
            (Self::Closed, _) | (_, Self::Closed) => Self::Closed,
            _ => Self::Adopted,
        }
    }
}

/// Records the restore on an error, in the same detail the GNOME rules use.
pub(crate) fn with_restore(mut error: AdapterError, restore: Restore) -> AdapterError {
    let mut details = match error.details.take() {
        Some(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    details.insert("list_restore".to_owned(), json!(restore.name()));
    error.with_details(Value::Object(details))
}
