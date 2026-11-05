use super::{count_of, split};

#[test]
fn the_first_separator_ends_the_application_name() {
    assert_eq!(
        split("Mail. Re: budget. final"),
        ("Mail".to_owned(), "Re: budget. final".to_owned())
    );
}

#[test]
fn a_name_without_a_summary_is_the_application_alone() {
    assert_eq!(split("Mail"), ("Mail".to_owned(), String::new()));
}

#[test]
fn a_tab_label_yields_the_count_in_its_trailing_parentheses() {
    assert_eq!(count_of("Current (12)"), Some(12));
    assert_eq!(count_of("Atual (0) "), Some(0));
}

#[test]
fn a_label_without_a_trailing_count_yields_none() {
    assert_eq!(count_of("Current"), None);
    assert_eq!(count_of("Current (many)"), None);
    assert_eq!(count_of("(3) Current"), None);
}
