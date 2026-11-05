use super::draws_natively;
use crate::system::hyprland::hypr_read::parse_clients;

const CLIENTS: &str = r#"[
  {"address":"0x1","mapped":true,"hidden":false,"at":[0,0],"size":[10,10],"workspace":{"id":1,"name":"1"},
   "floating":false,"pseudo":false,"monitor":0,"class":"a","title":"a","initialClass":"a","initialTitle":"a",
   "pid":10,"xwayland":true,"pinned":false,"fullscreen":0,"fullscreenClient":0,"overFullscreen":false,
   "grouped":[],"tags":[],"swallowing":"0x0","focusHistoryID":0,"inhibitingIdle":false,"xdgTag":"",
   "xdgDescription":"","contentType":"none","stableId":"1"},
  {"address":"0x2","mapped":true,"hidden":false,"at":[0,0],"size":[10,10],"workspace":{"id":1,"name":"1"},
   "floating":false,"pseudo":false,"monitor":0,"class":"b","title":"b","initialClass":"b","initialTitle":"b",
   "pid":20,"xwayland":false,"pinned":false,"fullscreen":0,"fullscreenClient":0,"overFullscreen":false,
   "grouped":[],"tags":[],"swallowing":"0x0","focusHistoryID":1,"inhibitingIdle":false,"xdgTag":"",
   "xdgDescription":"","contentType":"none","stableId":"2"}
]"#;

#[test]
fn only_a_process_with_an_xwayland_client_knows_its_screen_position() {
    let clients = parse_clients(CLIENTS).unwrap();
    assert!(!draws_natively(&clients, 10));
    assert!(draws_natively(&clients, 20));
    assert!(draws_natively(&clients, 30));
}
