use std::time::{Duration, Instant};

pub const QUIET_PERIOD: Duration = Duration::from_millis(200);
pub const RESTORE_TIMEOUT: Duration = Duration::from_secs(8);
pub const FAILED_INJECTION_TIMEOUT: Duration = Duration::from_millis(500);

pub struct State {
    pub published_at: Instant,
    pub injected_at: Option<Instant>,
    pub injection_failed: bool,
    pub receipts: Vec<Instant>,
    pub ownership_lost: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            published_at: Instant::now(),
            injected_at: None,
            injection_failed: false,
            receipts: Vec::new(),
            ownership_lost: false,
        }
    }

    pub fn last_receipt_after_injection(&self) -> Option<Instant> {
        let injected = self.injected_at?;
        self.receipts
            .iter()
            .copied()
            .rev()
            .find(|at| *at >= injected)
    }
}

pub fn should_finish(state: &State, now: Instant) -> bool {
    if state.ownership_lost {
        return true;
    }
    if let Some(last_receipt) = state.last_receipt_after_injection() {
        if now.duration_since(last_receipt) >= QUIET_PERIOD {
            return true;
        }
    }
    let timeout = if state.injection_failed {
        FAILED_INJECTION_TIMEOUT
    } else {
        RESTORE_TIMEOUT
    };
    now.duration_since(state.published_at) >= timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_published_ago(duration: Duration) -> State {
        let mut state = State::new();
        state.published_at = ago(duration);
        state
    }

    fn ago(duration: Duration) -> Instant {
        Instant::now()
            .checked_sub(duration)
            .unwrap_or_else(Instant::now)
    }

    #[test]
    fn waits_for_a_receipt() {
        let state = state_published_ago(Duration::from_millis(100));
        assert!(!should_finish(&state, Instant::now()));
    }

    #[test]
    fn ignores_receipts_before_injection() {
        let mut state = state_published_ago(Duration::from_millis(300));
        state.receipts.push(ago(Duration::from_millis(200)));
        state.injected_at = Some(ago(Duration::from_millis(100)));
        assert!(!should_finish(&state, Instant::now()));
    }

    #[test]
    fn finishes_after_receipts_go_quiet() {
        let mut state = state_published_ago(Duration::from_millis(300));
        state.injected_at = Some(ago(Duration::from_millis(250)));
        state.receipts.push(ago(QUIET_PERIOD));
        assert!(should_finish(&state, Instant::now()));
    }

    #[test]
    fn failed_injection_restores_quickly() {
        let mut state = state_published_ago(FAILED_INJECTION_TIMEOUT);
        state.injection_failed = true;
        assert!(should_finish(&state, Instant::now()));
    }

    #[test]
    fn ownership_loss_finishes_immediately() {
        let mut state = state_published_ago(Duration::from_millis(1));
        state.ownership_lost = true;
        assert!(should_finish(&state, Instant::now()));
    }
}
