use std::fmt::Write;

use super::virtual_key_home::home_code;
use crate::input::keyboard::keymap::Keymap;

const FIRST_CODE: u8 = 9;
const FIRST_SPARE_CODE: u8 = 136;
const LAST_CODE: u8 = 255;
const MODIFIER_ROWS: usize = 8;
const ROW_NAMES: [&str; MODIFIER_ROWS] = [
    "Shift", "Lock", "Control", "Mod1", "Mod2", "Mod3", "Mod4", "Mod5",
];
const SEEDED: [u32; 5] = [0xffe1, 0xffe3, 0xffe9, 0xffeb, 0xfe03];

/// The modifier row a keysym holds in the keymap, for the modifier keys a
/// key plan presses: Shift, Control, Alt or Meta, Super or Hyper, and
/// ISO Level 3 Shift.
fn modifier_row(keysym: u32) -> Option<usize> {
    match keysym {
        0xffe1 | 0xffe2 => Some(0),
        0xffe3 | 0xffe4 => Some(2),
        0xffe7..=0xffea => Some(3),
        0xffeb..=0xffee => Some(6),
        0xfe03 => Some(7),
        _ => None,
    }
}

/// The keyboard a virtual keyboard presents: one keycode per keysym, so any
/// keysym is typed by one key with no shift level. A keysym a standard
/// keyboard has takes that keyboard's keycode; any other takes a spare one.
/// Keycodes are X11 and xkb codes; the evdev code a virtual keyboard sends
/// is eight less.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VirtualKeyTable {
    keysyms: Vec<u32>,
}

impl Default for VirtualKeyTable {
    fn default() -> Self {
        let mut table = Self {
            keysyms: Vec::new(),
        };
        for keysym in SEEDED {
            table.insert(keysym);
        }
        table
    }
}

impl VirtualKeyTable {
    pub(crate) fn with(keysyms: impl IntoIterator<Item = u32>) -> Self {
        let mut table = Self::default();
        for keysym in keysyms {
            table.insert(keysym);
        }
        table
    }

    pub(crate) fn code_of(&self, keysym: u32) -> Option<u8> {
        let index = self.keysyms.iter().position(|known| *known == keysym)?;
        u8::try_from(index + usize::from(FIRST_CODE)).ok()
    }

    pub(crate) fn keysym_of(&self, code: u8) -> Option<u32> {
        let index = usize::from(code.checked_sub(FIRST_CODE)?);
        self.keysyms
            .get(index)
            .copied()
            .filter(|keysym| *keysym != 0)
    }

    /// Gives `keysym` a keycode, or `None` when every keycode is taken.
    /// The boolean says whether the table changed.
    pub(crate) fn insert(&mut self, keysym: u32) -> Option<(u8, bool)> {
        if let Some(code) = self.code_of(keysym) {
            return Some((code, false));
        }
        let code = home_code(keysym)
            .filter(|code| self.is_free(*code))
            .or_else(|| self.free_code())?;
        self.assign(code, keysym);
        Some((code, true))
    }

    fn is_free(&self, code: u8) -> bool {
        code >= FIRST_CODE && self.keysym_of(code).is_none()
    }

    /// A free keycode, spare codes first so the codes of a standard
    /// keyboard stay open for their own keysyms.
    fn free_code(&self) -> Option<u8> {
        (FIRST_SPARE_CODE..=LAST_CODE)
            .chain(FIRST_CODE..FIRST_SPARE_CODE)
            .find(|code| self.is_free(*code))
    }

    /// Puts `keysym` on `code`, for a plan that remaps a keycode.
    pub(crate) fn assign(&mut self, code: u8, keysym: u32) -> bool {
        let Some(index) = code.checked_sub(FIRST_CODE).map(usize::from) else {
            return false;
        };
        if index >= self.keysyms.len() {
            self.keysyms.resize(index + 1, 0);
        }
        self.keysyms[index] = keysym;
        true
    }

    /// The modifier mask bit a press of `code` sets, if it is a modifier.
    pub(crate) fn modifier_mask(&self, code: u8) -> Option<u32> {
        modifier_row(self.keysym_of(code)?).map(|row| 1 << row)
    }

    /// The table as the X11-shaped keymap the key and pointer planners read.
    pub(crate) fn keymap(&self) -> Keymap {
        let mut rows = vec![0_u8; MODIFIER_ROWS];
        for (index, keysym) in self.keysyms.iter().enumerate() {
            if let (Some(row), Ok(code)) = (
                modifier_row(*keysym),
                u8::try_from(index + usize::from(FIRST_CODE)),
            ) && rows[row] == 0
            {
                rows[row] = code;
            }
        }
        Keymap::new(FIRST_CODE, 1, self.keysyms.clone(), 1, rows)
    }

    /// The table as an xkb keymap for `zwp_virtual_keyboard_v1.keymap`. The
    /// types and compatibility sections come from the system's `complete`
    /// sets; symbols are written as raw keysym values.
    pub(crate) fn xkb_text(&self) -> String {
        let last = usize::from(FIRST_CODE) + self.keysyms.len().max(1) - 1;
        let mut text = String::from("xkb_keymap {\nxkb_keycodes \"agent-desktop\" {\n");
        let _ = writeln!(text, "minimum = 8;\nmaximum = {};", last.max(9));
        for index in 0..self.keysyms.len() {
            let code = index + usize::from(FIRST_CODE);
            let _ = writeln!(text, "<K{code}> = {code};");
        }
        text.push_str("};\nxkb_types \"agent-desktop\" { include \"complete\" };\n");
        text.push_str("xkb_compatibility \"agent-desktop\" { include \"complete\" };\n");
        text.push_str("xkb_symbols \"agent-desktop\" {\n");
        let mut members: Vec<Vec<String>> = vec![Vec::new(); MODIFIER_ROWS];
        for (index, keysym) in self.keysyms.iter().enumerate() {
            let code = index + usize::from(FIRST_CODE);
            if *keysym == 0 {
                continue;
            }
            let _ = writeln!(text, "key <K{code}> {{ [ 0x{keysym:x} ] }};");
            if let Some(row) = modifier_row(*keysym) {
                members[row].push(format!("<K{code}>"));
            }
        }
        for (row, keys) in members.iter().enumerate() {
            if !keys.is_empty() {
                let _ = writeln!(
                    text,
                    "modifier_map {} {{ {} }};",
                    ROW_NAMES[row],
                    keys.join(", ")
                );
            }
        }
        text.push_str("};\n};\n");
        text
    }
}

#[cfg(test)]
#[path = "virtual_key_table_tests.rs"]
mod tests;
