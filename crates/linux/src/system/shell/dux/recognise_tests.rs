use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::recognise;
use crate::system::shell::dux::procs::Procs;
use crate::system::shell::dux::proof::Proof;

const ME: u32 = 1000;

#[derive(Default)]
struct Session {
    executables: BTreeMap<u32, &'static str>,
    argv: BTreeMap<u32, Vec<&'static str>>,
    parents: BTreeMap<u32, u32>,
    users: BTreeMap<u32, u32>,
    files: BTreeSet<String>,
}

impl Session {
    fn process(&mut self, pid: u32, exe: &'static str, argv: Vec<&'static str>, user: u32) {
        self.executables.insert(pid, exe);
        self.argv.insert(pid, argv);
        self.users.insert(pid, user);
    }
}

impl Procs for Session {
    fn executable(&self, pid: u32) -> Option<String> {
        self.executables.get(&pid).map(|exe| (*exe).to_owned())
    }

    fn argv(&self, pid: u32) -> Option<Vec<String>> {
        self.argv
            .get(&pid)
            .map(|argv| argv.iter().map(|word| (*word).to_owned()).collect())
    }

    fn parent(&self, pid: u32) -> Option<u32> {
        self.parents.get(&pid).copied()
    }

    fn user(&self, pid: u32) -> Option<u32> {
        self.users.get(&pid).copied()
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.contains(path.to_str().unwrap_or_default())
    }
}

fn shell_files(session: &mut Session, directory: &str) {
    session.files.insert(format!("{directory}/DMSShell.qml"));
    session.files.insert(format!("{directory}/shell.qml"));
}

fn dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/home/u/.config"), PathBuf::from("/etc/xdg")]
}

#[test]
fn a_quickshell_that_loads_the_shells_own_directory_is_the_dux_shell() {
    let mut session = Session::default();
    session.process(
        50,
        "quickshell",
        vec!["qs", "-p", "/run/user/1000/dms-shell/abc"],
        ME,
    );
    shell_files(&mut session, "/run/user/1000/dms-shell/abc");

    assert_eq!(
        recognise(&session, 50, ME, &dirs()),
        Some(Proof::ConfigDirectory)
    );
}

#[test]
fn a_named_configuration_is_looked_up_in_every_configuration_directory() {
    let mut session = Session::default();
    session.process(50, "qs", vec!["qs", "-c", "dms"], ME);
    shell_files(&mut session, "/etc/xdg/quickshell/dms");

    assert_eq!(
        recognise(&session, 50, ME, &dirs()),
        Some(Proof::ConfigDirectory)
    );
}

#[test]
fn a_shell_file_path_names_its_directory() {
    let mut session = Session::default();
    session.process(50, "qs", vec!["qs", "-p", "/opt/dms/shell.qml"], ME);
    shell_files(&mut session, "/opt/dms");

    assert_eq!(
        recognise(&session, 50, ME, &dirs()),
        Some(Proof::ConfigDirectory)
    );
}

#[test]
fn a_quickshell_started_by_dms_run_is_the_dux_shell_even_from_an_unknown_directory() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs", "-p", "/somewhere/else"], ME);
    session.process(40, "dms", vec!["dms", "run", "--session"], ME);
    session.parents.insert(50, 40);

    assert_eq!(recognise(&session, 50, ME, &dirs()), Some(Proof::DmsParent));
}

#[test]
fn a_parent_that_is_not_running_the_shell_proves_nothing() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs", "-p", "/somewhere/else"], ME);
    session.process(40, "dms", vec!["dms", "version"], ME);
    session.parents.insert(50, 40);

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn another_quickshell_configuration_is_not_the_dux_shell() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs", "-c", "caelestia"], ME);
    session
        .files
        .insert("/home/u/.config/quickshell/caelestia/shell.qml".to_owned());

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn a_process_that_is_not_quickshell_is_never_the_dux_shell() {
    let mut session = Session::default();
    session.process(50, "gnome-shell", vec!["gnome-shell", "-p", "/opt/dms"], ME);
    shell_files(&mut session, "/opt/dms");

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn a_look_alike_executable_name_is_not_quickshell() {
    let mut session = Session::default();
    session.process(50, "quickshell-fake", vec!["qs", "-p", "/opt/dms"], ME);
    shell_files(&mut session, "/opt/dms");

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn another_users_quickshell_is_not_this_sessions_shell() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs", "-p", "/opt/dms"], ME + 1);
    shell_files(&mut session, "/opt/dms");

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn a_dms_parent_of_another_user_proves_nothing() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs"], ME);
    session.process(40, "dms", vec!["dms", "run"], ME + 1);
    session.parents.insert(50, 40);

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn a_directory_with_only_one_of_the_shells_files_is_not_enough() {
    let mut session = Session::default();
    session.process(50, "quickshell", vec!["qs", "-p", "/opt/dms"], ME);
    session.files.insert("/opt/dms/shell.qml".to_owned());

    assert_eq!(recognise(&session, 50, ME, &dirs()), None);
}

#[test]
fn an_unreadable_process_is_not_recognised() {
    assert_eq!(recognise(&Session::default(), 50, ME, &dirs()), None);
}
