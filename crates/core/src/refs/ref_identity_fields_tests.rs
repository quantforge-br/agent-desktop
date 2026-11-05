use super::super::{has_meaningful_identity, identity_fields_are_meaningful, identity_matches};

#[test]
fn identity_fields_report_own_evidence_and_ignore_a_retained_object() {
    assert!(!identity_fields_are_meaningful(
        "button", None, None, None, None
    ));
    assert!(!identity_fields_are_meaningful(
        "button",
        Some(""),
        None,
        Some(""),
        Some("")
    ));
    assert!(identity_fields_are_meaningful(
        "button",
        Some("Save"),
        None,
        None,
        None
    ));
    assert!(identity_fields_are_meaningful(
        "button",
        None,
        None,
        None,
        Some("save")
    ));
    assert!(identity_fields_are_meaningful(
        "textfield",
        Some("Search"),
        None,
        None,
        None
    ));
}

#[test]
fn a_slider_description_that_mirrors_its_value_is_not_identity() {
    let mut entry = super::entry();
    entry.identity.role = "slider".into();
    entry.identity.value = Some("50".into());
    entry.identity.description = Some("50".into());

    assert!(!has_meaningful_identity(&entry));
    assert!(!identity_fields_are_meaningful(
        "slider",
        None,
        Some("50"),
        Some("50.0"),
        None
    ));
    assert!(!identity_matches(
        &entry,
        None,
        Some("75"),
        Some("75"),
        None
    ));
}

#[test]
fn a_slider_description_that_is_a_label_stays_identity_across_value_changes() {
    let mut entry = super::entry();
    entry.identity.role = "slider".into();
    entry.identity.value = Some("50".into());
    entry.identity.description = Some("Volume".into());

    assert!(has_meaningful_identity(&entry));
    assert!(identity_matches(
        &entry,
        None,
        Some("75"),
        Some("Volume"),
        None
    ));
    assert!(!identity_matches(
        &entry,
        None,
        Some("75"),
        Some("Brightness"),
        None
    ));
}

#[test]
fn a_recorded_name_or_description_survives_a_value_write_that_equals_it() {
    let mut entry = super::entry();
    entry.identity.role = "textfield".into();
    entry.identity.name = Some("Search".into());
    entry.identity.value = Some(String::new());

    assert!(identity_matches(
        &entry,
        Some("Search"),
        Some("Search"),
        None,
        None
    ));

    entry.identity.name = None;
    entry.identity.description = Some("Search".into());

    assert!(identity_matches(
        &entry,
        None,
        Some("Search"),
        Some("Search"),
        None
    ));
}
