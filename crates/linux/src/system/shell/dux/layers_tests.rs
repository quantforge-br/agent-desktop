use super::parse;

const REPLY: &str = r#"{
"WAYLAND-1": { "levels": {
  "0": [ {"address":"0x1","x":0,"y":0,"w":1280,"h":720,"alpha":1,"namespace":"quickshell","pid":50} ],
  "1": [],
  "2": [ {"address":"0x2","x":0,"y":646,"w":1280,"h":74,"alpha":1,"namespace":"dms:dock","pid":50},
         {"address":"0x3","x":0,"y":0,"w":712,"h":720,"alpha":1,"namespace":"dms:control-center","pid":50} ]
}}}"#;

#[test]
fn layers_of_every_level_are_listed_with_corner_size_and_owner() {
    let layers = parse(REPLY).expect("parsed");

    let seen: Vec<_> = layers
        .iter()
        .map(|layer| {
            (
                layer.namespace.as_str(),
                (layer.x, layer.y),
                (layer.width, layer.height),
                layer.pid,
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            ("quickshell", (0, 0), (1280, 720), 50),
            ("dms:dock", (0, 646), (1280, 74), 50),
            ("dms:control-center", (0, 0), (712, 720), 50),
        ]
    );
}

#[test]
fn an_output_with_no_layers_yields_an_empty_list() {
    assert!(
        parse(r#"{"WAYLAND-1":{"levels":{"0":[],"1":[]}}}"#)
            .expect("parsed")
            .is_empty()
    );
}

#[test]
fn a_reply_that_is_not_the_layer_list_is_an_error_not_an_empty_list() {
    assert!(parse("not json").is_err());
    assert!(parse(r#"{"WAYLAND-1":{}}"#).is_err());
    assert!(parse(r#"{"WAYLAND-1":{"levels":{"0":[{"namespace":"x"}]}}}"#).is_err());
}
