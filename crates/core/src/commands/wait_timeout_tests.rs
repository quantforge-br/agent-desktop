use super::*;

#[test]
fn text_timeout_reports_the_char_count_and_never_the_raw_text() {
    let marker = "zzq93f_super_secret_marker_do_not_leak";

    let AppError::Adapter(error) = text(marker, 50, None, None).unwrap_err() else {
        panic!("adapter error expected")
    };

    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(!error.message.contains(marker));
    let details = error.details.expect("timeout details");
    assert!(!details.to_string().contains(marker));
    assert_eq!(details["predicate"], "text");
    assert_eq!(details["text_chars"], marker.chars().count());
}

#[test]
fn a_missing_window_is_not_blamed_on_a_busy_app() {
    let AppError::Adapter(error) = window("Nope", None, 50, None, None).unwrap_err() else {
        panic!("adapter error expected")
    };

    let suggestion = error.suggestion.expect("timeout suggestion");
    assert!(!suggestion.contains("busy or unresponsive"), "{suggestion}");
    assert!(suggestion.contains("list-windows"), "{suggestion}");
}
