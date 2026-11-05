use super::permission_probe_deadline;
use crate::cli::Cli;
use clap::Parser;

const REF: &str = "@s8f3k2p9:e1";

fn probe_ms(argv: &[&str]) -> u64 {
    let cli = Cli::try_parse_from(std::iter::once("agent-desktop").chain(argv.iter().copied()))
        .expect("the command parses");
    let command = cli.command.expect("a subcommand");
    permission_probe_deadline(&command)
        .expect("a deadline builds")
        .timeout_ms()
}

#[test]
fn a_short_command_timeout_bounds_the_permission_probe_but_never_below_one_second() {
    let cases: [(&[&str], u64); 7] = [
        (&["snapshot", "--app", "X"], 3_000),
        (&["snapshot", "--app", "X", "--timeout-ms", "300"], 1_000),
        (&["click", REF, "--timeout-ms", "2500"], 2_500),
        (&["type", REF, "hi", "--timeout-ms", "1500"], 1_500),
        (&["find", "--app", "X", "--timeout-ms", "1200"], 1_200),
        (&["wait", "--window", "Doc", "--timeout", "1800"], 1_800),
        (&["batch", "[]", "--timeout-ms", "2000"], 2_000),
    ];
    for (argv, expected) in cases {
        assert_eq!(probe_ms(argv), expected, "{argv:?}");
    }
}

#[test]
fn the_permission_probe_keeps_five_seconds_without_a_shorter_command_timeout() {
    let cases: [&[&str]; 5] = [
        &["wait", "--window", "Doc"],
        &["click", REF, "--timeout-ms", "0"],
        &["wait", "200"],
        &["launch", "gedit", "--timeout", "60000"],
        &["press", "a"],
    ];
    for argv in cases {
        assert_eq!(probe_ms(argv), 5_000, "{argv:?}");
    }
}
