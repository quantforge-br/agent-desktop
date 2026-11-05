use super::clap_error_summary;

/// The shape clap emits for a missing positional: the headline carries no
/// argument name, so a summary built from it alone cannot say what is missing.
const MISSING_POSITIONAL: &str = "error: the following required arguments were not provided:\n  <VALUE>\n\nUsage: agent-desktop.exe set-value <REF> <VALUE>\n\nFor more information, try '--help'.";

#[test]
fn a_missing_argument_summary_names_the_argument() {
    let summary = clap_error_summary(MISSING_POSITIONAL);

    assert!(
        summary.contains("<VALUE>"),
        "the caller cannot act on a message that omits the missing argument, got: {summary}"
    );
    assert!(
        !summary.contains("Usage:"),
        "usage boilerplate is not actionable for a machine consumer, got: {summary}"
    );
    assert!(
        !summary.contains('\n'),
        "the envelope carries a single-line message, got: {summary}"
    );
}

#[test]
fn a_single_line_error_is_unchanged_and_an_empty_one_still_says_something() {
    assert_eq!(
        clap_error_summary("error: unexpected argument '--nope' found"),
        "error: unexpected argument '--nope' found"
    );
    assert_eq!(clap_error_summary(""), "parse error");
    assert_eq!(clap_error_summary("\n\nUsage: x"), "parse error");
}

fn parse_failure(args: &[&str]) -> (String, String) {
    let argv = std::iter::once("agent-desktop").chain(args.iter().copied());
    let error = crate::root_command()
        .try_get_matches_from(argv)
        .expect_err("these arguments must not parse");
    let command = super::clap_command_name(args.iter().map(|arg| (*arg).to_owned()));
    (command, clap_error_summary(&error.to_string()))
}

#[test]
fn a_negative_option_value_is_told_to_use_equals() {
    let (command, summary) = parse_failure(&["move-window", "--x", "-100", "--y", "20"]);

    let hint = super::clap_parse_hint(&command, &summary).expect("a hint for a negative number");

    assert!(hint.contains("--x=-100"), "got: {hint}");
}

#[test]
fn a_dash_leading_positional_is_told_to_use_double_dash() {
    let (command, summary) = parse_failure(&["clipboard-set", "-x"]);

    let hint = super::clap_parse_hint(&command, &summary).expect("a hint for dash text");

    assert!(hint.contains("clipboard-set -- -x"), "got: {hint}");
}

#[test]
fn launch_app_arguments_are_told_to_use_the_arg_flag() {
    let (command, summary) = parse_failure(&["launch", "zenity", "TRUE"]);

    let hint = super::clap_parse_hint(&command, &summary).expect("a hint for launch");

    assert!(hint.contains("--arg=VALUE"), "got: {hint}");
}

#[test]
fn other_parse_failures_get_no_hint() {
    let (command, summary) = parse_failure(&["click"]);

    assert_eq!(super::clap_parse_hint(&command, &summary), None);
}
