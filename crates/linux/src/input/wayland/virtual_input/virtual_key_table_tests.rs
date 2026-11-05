use agent_desktop_core::Modifier;

use super::VirtualKeyTable;

#[test]
fn every_keysym_gets_its_own_unshifted_keycode() {
    let table = VirtualKeyTable::with([0x41, 0x61, 0x20ac, 0x0100_00e9]);
    let codes: Vec<u8> = [0x41, 0x61, 0x20ac, 0x0100_00e9]
        .iter()
        .map(|keysym| table.code_of(*keysym).unwrap())
        .collect();
    let mut unique = codes.clone();
    unique.dedup();
    assert_eq!(codes, unique);
    let map = table.keymap();
    let lookup = map.lookup(0x41).unwrap();
    assert!(!lookup.shift);
    assert_eq!(lookup.code, table.code_of(0x41).unwrap());
}

#[test]
fn the_planner_keymap_names_one_key_for_each_modifier_row() {
    let map = VirtualKeyTable::default().keymap();
    for modifier in [
        Modifier::Shift,
        Modifier::Ctrl,
        Modifier::Alt,
        Modifier::Meta,
    ] {
        assert!(map.modifier_code(&modifier).is_some(), "{modifier:?}");
    }
    let table = VirtualKeyTable::default();
    let control = map.modifier_code(&Modifier::Ctrl).unwrap();
    assert_eq!(table.modifier_mask(control), Some(4));
    let super_key = map.modifier_code(&Modifier::Meta).unwrap();
    assert_eq!(table.modifier_mask(super_key), Some(64));
}

#[test]
fn a_full_table_refuses_another_keysym_instead_of_reusing_a_code() {
    let mut table = VirtualKeyTable::default();
    let mut keysym = 0x0100_0100;
    while table.insert(keysym).is_some() {
        keysym += 1;
    }
    assert_eq!(table.code_of(keysym), None);
    assert_eq!(table.insert(0x41), None);
    assert_eq!(table.insert(0xffe1), Some((50, false)));
}

#[test]
fn a_keysym_of_a_standard_keyboard_takes_its_keycode_whatever_came_first() {
    let table = VirtualKeyTable::with([0x20ac, 0x0100_00e9, 0x61, 0xff50, 0xff08, 0x41]);
    assert_eq!(table.code_of(0x61), Some(38));
    assert_eq!(table.code_of(0xff50), Some(110));
    assert_eq!(table.code_of(0xff08), Some(22));
    assert_eq!(table.code_of(0xffe3), Some(37));
    for spare in [0x20ac, 0x0100_00e9, 0x41] {
        assert!(
            table.code_of(spare).is_some_and(|code| code >= 136),
            "{spare:x}"
        );
    }
}

#[test]
fn a_keysym_without_a_spare_code_left_takes_a_free_standard_code() {
    let mut table = VirtualKeyTable::default();
    for keysym in 0x0100_0100..0x0100_0100 + 120 {
        assert!(table.insert(keysym).is_some_and(|(code, _)| code >= 136));
    }
    let (code, changed) = table.insert(0x0100_0200).unwrap();
    assert!(changed);
    assert!(code < 136);
    assert_eq!(table.keysym_of(code), Some(0x0100_0200));
}

#[test]
fn the_xkb_text_maps_each_code_and_its_modifier_role() {
    let table = VirtualKeyTable::with([0x61]);
    let code = table.code_of(0x61).unwrap();
    let text = table.xkb_text();
    assert!(text.contains(&format!("<K{code}> = {code};")));
    assert!(text.contains(&format!("key <K{code}> {{ [ 0x61 ] }};")));
    assert!(text.contains("modifier_map Control { <K37> };"));
    assert!(text.contains("modifier_map Mod4 { <K133> };"));
}
