use agent_desktop_core::{ProcessId, WindowInfo, WindowState};

use super::taken_from_a_shell;
use crate::system::shell::dux::backend::DuxShell;
use crate::system::shell::dux::proof::Proof;

fn window(app: &str, pid: u32) -> WindowInfo {
    WindowInfo {
        id: "w-1".into(),
        title: String::new(),
        app: app.into(),
        pid: ProcessId::new(pid),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    }
}

#[test]
fn only_a_quickshell_window_is_held_to_the_window_it_names() {
    let shell = DuxShell {
        pid: 7,
        proof: Proof::DmsParent,
    };

    assert!(taken_from_a_shell(shell, &window("quickshell", 7)));
    assert!(taken_from_a_shell(shell, &window("quickshell", 9)));
    assert!(!taken_from_a_shell(shell, &window("gnome-calculator", 9)));
}
