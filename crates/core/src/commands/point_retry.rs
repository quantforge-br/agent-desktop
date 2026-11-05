use crate::{
    AppError, ErrorCode,
    actionability::pointer_timeout::point_actionability_timeout,
    commands::pointer_action::{is_retryable_point_error, point_error_report},
    refs::ref_resolve_deadline::POLL_INTERVAL,
};

/// Runs a leased pointer phase once, or with a positive timeout again after
/// each retryable actionability failure until the deadline. A deadline that
/// runs out after such a failure ends the phase with that failure as the
/// timeout's `last_report`, never with the lease or resolution timeout a
/// further attempt would meet.
pub(crate) fn retry_leased_point_phase<T>(
    timeout_ms: Option<u64>,
    deadline: crate::Deadline,
    mut attempt: impl FnMut() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let auto_wait = timeout_ms.is_some_and(|timeout_ms| timeout_ms > 0);
    let mut last_report = None;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(error) if auto_wait && is_retryable_point_error(&error) => {
                last_report = Some(point_error_report(&error));
                let remaining = deadline.remaining();
                if remaining.is_zero() {
                    return Err(point_actionability_timeout(last_report));
                }
                std::thread::sleep(POLL_INTERVAL.min(remaining));
                if deadline.is_expired() {
                    return Err(point_actionability_timeout(last_report));
                }
            }
            Err(error) if last_report.is_some() && deadline.is_expired() && is_timeout(&error) => {
                return Err(point_actionability_timeout(last_report));
            }
            Err(error) => return Err(error),
        }
    }
}

fn is_timeout(error: &AppError) -> bool {
    matches!(error, AppError::Adapter(error) if error.code == ErrorCode::Timeout)
}
