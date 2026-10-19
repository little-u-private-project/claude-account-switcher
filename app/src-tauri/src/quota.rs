// Plan usage per account, polled in the background (lib.rs) so the menu bar
// and notifications work while the window is hidden. Read-only, like
// token.rs: a stopped account's token expires and only its own Claude can
// refresh it, so "expired" is a state of its own, not "signed out".

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UsageState {
    Ok { usage: Value },
    SignedOut,
    Expired,
    Error { message: String },
}

impl UsageState {
    fn pct(&self, window: &str) -> Option<f64> {
        match self {
            UsageState::Ok { usage } => usage[window]["utilization"].as_f64(),
            _ => None,
        }
    }

    // The limit that bites first: the fuller of the two windows.
    pub fn binding(&self) -> Option<f64> {
        match (self.pct("five_hour"), self.pct("seven_day")) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }

    pub fn short(&self) -> String {
        match self {
            UsageState::Ok { .. } => {
                let p = |w| self.pct(w).map_or("–".into(), |v| format!("{v:.0}%"));
                format!("5h {} · 7d {}", p("five_hour"), p("seven_day"))
            }
            UsageState::SignedOut => "sign in needed".into(),
            UsageState::Expired => "open to refresh".into(),
            UsageState::Error { .. } => "usage unavailable".into(),
        }
    }
}

pub static CACHE: LazyLock<Mutex<HashMap<String, UsageState>>> = LazyLock::new(Default::default);

pub async fn fetch(id: &str) -> UsageState {
    let dir = match crate::store::user_data_dir(id) {
        Ok(d) => d,
        Err(message) => return UsageState::Error { message },
    };
    match crate::token::usage(&dir).await {
        Ok(usage) => UsageState::Ok { usage },
        Err(e) if e == crate::token::NOT_SIGNED_IN => UsageState::SignedOut,
        Err(e) if e == crate::token::EXPIRED => UsageState::Expired,
        Err(message) => UsageState::Error { message },
    }
}

pub const WINDOWS: [(&str, &str); 2] = [("five_hour", "5-hour"), ("seven_day", "weekly")];
pub const NEAR_LIMIT: f64 = 90.0;

// Some(true) when a window just went past NEAR_LIMIT, Some(false) when it
// just came back under it (reset); None otherwise - including the first
// reading, so starting the app doesn't fire a burst of notifications.
pub fn crossing(was_high: Option<bool>, pct: f64) -> Option<bool> {
    let high = pct >= NEAR_LIMIT;
    (was_high? != high).then_some(high)
}

// Per (account, window): over NEAR_LIMIT at the last reading?
pub static LEVELS: LazyLock<Mutex<HashMap<(String, &'static str), bool>>> = LazyLock::new(Default::default);

// Most room left = lowest binding %; a tie goes to one already running.
// Accounts without a usage reading can't be ranked.
pub fn pick_best<'a>(candidates: impl IntoIterator<Item = (&'a str, bool, Option<f64>)>) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter_map(|(id, running, pct)| pct.map(|p| (id, running, p)))
        .min_by(|a, b| a.2.total_cmp(&b.2).then(b.1.cmp(&a.1)))
        .map(|(id, _, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_fires_only_on_change_after_first_reading() {
        assert_eq!(crossing(None, 95.0), None);
        assert_eq!(crossing(Some(false), 89.9), None);
        assert_eq!(crossing(Some(false), 90.0), Some(true));
        assert_eq!(crossing(Some(true), 99.0), None);
        assert_eq!(crossing(Some(true), 3.0), Some(false));
    }

    #[test]
    fn best_is_lowest_known_then_running() {
        assert_eq!(pick_best([("a", false, Some(50.0)), ("b", false, None), ("c", true, Some(20.0))]), Some("c"));
        assert_eq!(pick_best([("a", false, Some(20.0)), ("b", true, Some(20.0))]), Some("b"));
        assert_eq!(pick_best([("a", false, None)]), None);
    }
}
