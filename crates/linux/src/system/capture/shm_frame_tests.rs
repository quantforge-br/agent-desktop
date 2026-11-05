use super::{choose_format, to_rgb};

#[test]
fn little_endian_rgb_words_put_blue_first_in_memory() {
    let pixels = [0x10, 0x20, 0x30, 0xff, 0x01, 0x02, 0x03, 0x00];
    assert_eq!(
        to_rgb(&pixels, 1, 2, 1).unwrap(),
        vec![0x30, 0x20, 0x10, 0x03, 0x02, 0x01]
    );
}

#[test]
fn bgr_words_keep_red_first() {
    let pixels = [0x10, 0x20, 0x30, 0xff];
    assert_eq!(
        to_rgb(&pixels, 0x3432_4258, 1, 1).unwrap(),
        vec![0x10, 0x20, 0x30]
    );
}

#[test]
fn a_short_buffer_or_a_ten_bit_format_is_not_read() {
    assert_eq!(to_rgb(&[0; 4], 1, 2, 1), None);
    assert_eq!(to_rgb(&[0; 4], 0x3033_5258, 1, 1), None);
}

#[test]
fn an_eight_bit_format_is_chosen_over_a_ten_bit_one_offered_first() {
    assert_eq!(choose_format(&[0x3033_5258, 0x3432_4258, 1]), Some(1));
    assert_eq!(choose_format(&[0x3033_5258]), None);
}
