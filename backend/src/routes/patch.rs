//! The shared PATCH-merge helpers: absent fields keep the existing value.

/// `Some` wins, `None` keeps `existing`.
pub fn merge<T>(patch: Option<T>, existing: T) -> T {
    patch.unwrap_or(existing)
}

/// For optional values a patch can't null out: `Some` wins, `None` keeps
/// `existing`.
pub fn merge_opt<T>(patch: Option<T>, existing: Option<T>) -> Option<T> {
    patch.or(existing)
}

/// For nullable numbers: `None` keeps `existing`, `Some(None)` clears it.
pub fn merge_num(patch: Option<Option<f64>>, existing: Option<f64>) -> Option<f64> {
    match patch {
        Some(value) => value,
        None => existing,
    }
}

/// For nullable text: `None` keeps `existing`; a present value is trimmed and
/// `null`/blank clears it.
pub fn merge_text(patch: Option<Option<String>>, existing: Option<String>) -> Option<String> {
    match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    }
}
