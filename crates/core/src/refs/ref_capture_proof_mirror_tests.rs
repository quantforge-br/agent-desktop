use super::{
    CaptureProof, LocatorEvidence, LocatorField, RefMap, allocate_refs, config, evidence, named,
    observed_from, window,
};
use crate::AccessibilityNode;
use crate::IdentityMatch;
use crate::refs::ref_full_predicate::FullPredicate;
use crate::refs::ref_full_predicate::{DESCRIPTION, NAME};
use crate::refs::ref_live_predicate::LivePredicate;

fn slider_node(description: &str, value: &str) -> AccessibilityNode {
    let mut slider = named("slider", "");
    slider.identity.name = None;
    slider.identity.description = Some(description.into());
    slider.identity.value = Some(value.into());
    slider
}

fn slider_evidence(description: &str, value: &str) -> LocatorEvidence {
    let mut live = evidence("slider", "", Vec::new(), 0);
    live.name = LocatorField::Absent;
    live.description = LocatorField::Known(description.into());
    live.value = LocatorField::Known(value.into());
    live
}

fn captured_slider(description: &str, value: &str) -> crate::RefEntry {
    let root = window(vec![slider_node(description, value)]);
    let observed = observed_from(root.clone());
    let proof = CaptureProof::new(&observed);
    let mut refmap = RefMap::new();
    allocate_refs(root, &mut refmap, &config(&proof)).unwrap();
    (1..=refmap.len())
        .filter_map(|index| refmap.get(&format!("@e{index}")))
        .find(|entry| entry.identity.role == "slider")
        .cloned()
        .expect("the slider is allocated a ref")
}

#[test]
fn a_description_that_mirrors_the_value_does_not_make_the_ref_stale_after_a_set_value() {
    let entry = captured_slider("50", "50");
    let predicate = FullPredicate::of_entry(&entry);

    assert_eq!(predicate.mask() & DESCRIPTION, 0);
    let after_set_value = slider_evidence("75", "75");
    assert_eq!(
        predicate.compare(&LivePredicate::observed(&after_set_value)),
        IdentityMatch::Match
    );
}

#[test]
fn a_description_that_differs_from_the_value_still_identifies_the_slider() {
    let entry = captured_slider("Volume", "50");
    let predicate = FullPredicate::of_entry(&entry);

    assert_ne!(predicate.mask() & DESCRIPTION, 0);
    assert_eq!(
        predicate.compare(&LivePredicate::observed(&slider_evidence("Volume", "75"))),
        IdentityMatch::Match
    );
    assert_eq!(
        predicate.compare(&LivePredicate::observed(&slider_evidence(
            "Brightness",
            "75"
        ))),
        IdentityMatch::NoMatch
    );
}

fn captured_textfield(name: &str, description: Option<&str>) -> crate::RefEntry {
    let mut field = named("textfield", name);
    field.identity.description = description.map(Into::into);
    field.identity.value = Some(String::new());
    let root = window(vec![field]);
    let observed = observed_from(root.clone());
    let proof = CaptureProof::new(&observed);
    let mut refmap = RefMap::new();
    allocate_refs(root, &mut refmap, &config(&proof)).unwrap();
    (1..=refmap.len())
        .filter_map(|index| refmap.get(&format!("@e{index}")))
        .find(|entry| entry.identity.role == "textfield")
        .cloned()
        .expect("the textfield is allocated a ref")
}

fn textfield_evidence(name: &str, description: Option<&str>, value: &str) -> LocatorEvidence {
    let mut live = evidence("textfield", name, Vec::new(), 0);
    live.description = description.map_or(LocatorField::Absent, |text| {
        LocatorField::Known(text.into())
    });
    live.value = LocatorField::Known(value.into());
    live
}

#[test]
fn a_recorded_description_survives_a_value_write_that_equals_it() {
    let entry = captured_textfield("Query", Some("Search"));
    let predicate = FullPredicate::of_entry(&entry);

    assert_ne!(predicate.mask() & DESCRIPTION, 0);
    let after_typing = textfield_evidence("Query", Some("Search"), "Search");
    assert_eq!(
        predicate.compare(&LivePredicate::observed(&after_typing)),
        IdentityMatch::Match
    );
}

#[test]
fn a_recorded_name_survives_a_value_write_that_equals_it() {
    let entry = captured_textfield("Search", None);
    let predicate = FullPredicate::of_entry(&entry);

    assert_ne!(predicate.mask() & NAME, 0);
    let after_typing = textfield_evidence("Search", None, "Search");
    assert_eq!(
        predicate.compare(&LivePredicate::observed(&after_typing)),
        IdentityMatch::Match
    );
}
