const SEPARATOR: &str = ". ";

/// Splits a card's accessible name into the application and the summary. The
/// shell joins the two with a full stop and a space and leaves out an empty
/// part, so the first separator ends the application name; a name without
/// one is the application alone.
pub(super) fn split(name: &str) -> (String, String) {
    match name.split_once(SEPARATOR) {
        Some((app, summary)) => (app.to_owned(), summary.to_owned()),
        None => (name.to_owned(), String::new()),
    }
}

/// The count a tab label carries in trailing parentheses, `Current (4)`.
pub(super) fn count_of(label: &str) -> Option<usize> {
    let inner = label.trim_end().strip_suffix(')')?;
    let (_, digits) = inner.rsplit_once('(')?;
    digits.trim().parse().ok()
}

#[cfg(test)]
#[path = "name_tests.rs"]
mod tests;
