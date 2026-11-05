use super::reject;
use crate::{AppError, DeliverySemantics};

#[test]
fn held_input_refusal_states_that_nothing_was_sent_and_retry_is_safe() {
    let AppError::Adapter(error) = reject("key-down", "press") else {
        panic!("held input refusal must be an adapter error");
    };

    assert_eq!(error.code.as_str(), "ACTION_NOT_SUPPORTED");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(error.details.unwrap()["raw_input_emitted"], false);
}
