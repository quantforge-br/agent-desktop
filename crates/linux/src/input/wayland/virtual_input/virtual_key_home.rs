//! Where a keysym sits on a standard evdev keyboard.

const EVDEV_OFFSET: u16 = 8;

/// The xkb keycode `keysym` has on a standard evdev keyboard with a QWERTY
/// letter block, when it has one. A virtual keyboard puts the keysym there
/// because GTK 3 matches its key bindings by keycode against the keymap it
/// read first and does not follow a later one: Home, BackSpace or Ctrl+A
/// only reach a GTK 3 binding on the keycode the seat's own keyboard uses.
pub(crate) fn home_code(keysym: u32) -> Option<u8> {
    let evdev = letter_code(keysym)
        .or_else(|| digit_code(keysym))
        .or_else(|| symbol_code(keysym))
        .or_else(|| named_code(keysym))?;
    u8::try_from(evdev + EVDEV_OFFSET).ok()
}

fn letter_code(keysym: u32) -> Option<u16> {
    const ROWS: [(&[u8], u16); 3] = [(b"qwertyuiop", 16), (b"asdfghjkl", 30), (b"zxcvbnm", 44)];
    let letter = u8::try_from(keysym).ok()?;
    ROWS.iter().find_map(|(row, first)| {
        let column = row.iter().position(|key| *key == letter)?;
        u16::try_from(column).ok().map(|column| first + column)
    })
}

fn digit_code(keysym: u32) -> Option<u16> {
    match keysym {
        0x30 => Some(11),
        0x31..=0x39 => u16::try_from(keysym - 0x31 + 2).ok(),
        _ => None,
    }
}

fn symbol_code(keysym: u32) -> Option<u16> {
    Some(match keysym {
        0x20 => 57,
        0x27 => 40,
        0x2c => 51,
        0x2d => 12,
        0x2e => 52,
        0x2f => 53,
        0x3b => 39,
        0x3d => 13,
        0x5b => 26,
        0x5c => 43,
        0x5d => 27,
        0x60 => 41,
        _ => return None,
    })
}

fn named_code(keysym: u32) -> Option<u16> {
    Some(match keysym {
        0xff1b => 1,
        0xff08 => 14,
        0xff09 => 15,
        0xff0d => 28,
        0xffe3 => 29,
        0xffe1 => 42,
        0xffe2 => 54,
        0xffe9 => 56,
        0xffe5 => 58,
        0xffbe..=0xffc7 => return u16::try_from(keysym - 0xffbe + 59).ok(),
        0xff7f => 69,
        0xff14 => 70,
        0xfe03 => 84,
        0xffc8 => 87,
        0xffc9 => 88,
        0xff8d => 96,
        0xffe4 => 97,
        0xff61 => 99,
        0xffea => 100,
        0xff50 => 102,
        0xff52 => 103,
        0xff55 => 104,
        0xff51 => 105,
        0xff53 => 106,
        0xff57 => 107,
        0xff54 => 108,
        0xff56 => 109,
        0xff63 => 110,
        0xffff => 111,
        0xff13 => 119,
        0xffeb => 125,
        0xffec => 126,
        0xff67 => 127,
        _ => return None,
    })
}
