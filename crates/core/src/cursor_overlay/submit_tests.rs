use super::super::{
    CursorOverlayConfig, CursorOverlayControl, CursorOverlayInstruction, CursorPhase,
};
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{AdapterError, CommandContext, ErrorCode};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

struct LevelCapture(Arc<Mutex<Vec<Level>>>);

impl Subscriber for LevelCapture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        event.record(&mut NoFields);
        self.0.lock().unwrap().push(*event.metadata().level());
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

struct NoFields;

impl Visit for NoFields {
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}

struct RefusingOverlay(ErrorCode);

impl ObservationOps for RefusingOverlay {}
impl ActionOps for RefusingOverlay {}
impl InputOps for RefusingOverlay {}

impl SystemOps for RefusingOverlay {
    fn update_cursor_overlay(&self, _control: &CursorOverlayControl) -> Result<(), AdapterError> {
        Err(AdapterError::new(self.0.clone(), "overlay refused"))
    }
}

fn levels_logged_when_overlay_refuses(code: ErrorCode) -> Vec<Level> {
    let config = CursorOverlayConfig::enabled(None, 6).expect("valid config");
    let context = CommandContext::default().with_cursor_overlay_session("test-session", config);
    let events = Arc::new(Mutex::new(Vec::new()));
    tracing::subscriber::with_default(LevelCapture(events.clone()), || {
        super::submit_caret(
            &RefusingOverlay(code),
            &context,
            crate::Rect {
                x: 1.0,
                y: 2.0,
                width: 2.0,
                height: 10.0,
            },
        );
    });
    events.lock().unwrap().clone()
}

#[test]
fn an_overlay_the_platform_cannot_draw_does_not_warn_on_every_action() {
    let levels = levels_logged_when_overlay_refuses(ErrorCode::PlatformNotSupported);

    assert!(!levels.is_empty());
    assert!(levels.iter().all(|level| *level > Level::WARN));
}

#[test]
fn an_overlay_that_fails_for_another_reason_still_warns() {
    let levels = levels_logged_when_overlay_refuses(ErrorCode::Internal);

    assert!(levels.contains(&Level::WARN));
}

#[test]
fn a_caret_instruction_round_trips_through_the_overlay_transport() {
    let config = CursorOverlayConfig::enabled(None, 6).expect("valid config");
    let caret = crate::Rect {
        x: 50.0,
        y: 78.0,
        width: 2.0,
        height: 25.0,
    };
    let instruction =
        CursorOverlayInstruction::new(crate::Point { x: 50.0, y: 90.5 }, &config, false)
            .expect("valid instruction")
            .with_target(Some(caret))
            .with_phase(CursorPhase::Caret);
    let control = CursorOverlayControl::present_with_style(
        "run-1".into(),
        instruction,
        config.style().clone(),
    );
    let wire = serde_json::to_vec(&control).expect("encodes");
    let decoded: CursorOverlayControl = serde_json::from_slice(&wire).expect("decodes");
    assert_eq!(decoded, control);
    assert!(decoded.expects_acknowledgement());
}
