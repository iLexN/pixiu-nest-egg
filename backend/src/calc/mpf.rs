use super::*;

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::MpfFigures;

// ----- MPF (強積金) -----

/// A recorded MPF account state: a history row, or the standing values.
#[derive(Debug, Clone, Copy)]
pub struct MpfPoint {
    pub recorded_on: chrono::NaiveDate,
    pub contributions: f64,
    pub balance: f64,
}

/// Everything needed to derive one account's figures.
#[derive(Debug, Clone)]
pub struct MpfAccountFacts {
    /// Creation date of the account record: the as-of fallback boundary —
    /// dates at or after it fall back to the current values when no history
    /// row is old enough.
    pub created_on: chrono::NaiveDate,
    pub contributions: f64,
    pub balance: f64,
    pub seed_max_rate: Option<f64>,
    pub seed_max_gain: Option<f64>,
    pub history: Vec<MpfPoint>,
}

/// Section-level aggregates for the MPF overview header.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfTotals {
    /// Σ contributions (the sheet's `buy`).
    pub buy: f64,
    /// Σ balance (the sheet's `now`).
    pub now: f64,
    pub rate: Option<f64>,
    pub gain: f64,
    pub last_month: Option<MpfFigures>,
    pub max: MpfFigures,
}

/// An MPF account as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct MpfAccountInput<'a> {
    pub label: &'a str,
    pub contributions: f64,
    pub balance: f64,
}

pub fn validate_mpf_account(input: MpfAccountInput<'_>) -> Result<(), Vec<FieldError>> {
    let mut errors = Vec::new();
    if input.label.trim().is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if input.contributions < 0.0 {
        errors.push(FieldError::new(
            "contributions",
            "contributions must not be negative",
        ));
    }
    if input.balance < 0.0 {
        errors.push(FieldError::new("balance", "balance must not be negative"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// rate = (balance − contributions) ÷ contributions; empty at contributions 0.
pub fn mpf_figures(contributions: f64, balance: f64) -> MpfFigures {
    MpfFigures {
        rate: (contributions > 0.0).then(|| (balance - contributions) / contributions),
        gain: balance - contributions,
    }
}

/// Figures of the latest history row dated on or before the last day of the
/// previous calendar month — an as-of lookup, so a month with no recorded
/// row still reports the figures that stood at its end.
pub fn mpf_last_month(history: &[MpfPoint], today: chrono::NaiveDate) -> Option<MpfFigures> {
    let prev_end = first_of_month(today) - chrono::Days::new(1);
    history
        .iter()
        .filter(|point| point.recorded_on <= prev_end)
        .max_by_key(|point| point.recorded_on)
        .map(|point| mpf_figures(point.contributions, point.balance))
}

fn fold_max(acc: Option<f64>, value: Option<f64>) -> Option<f64> {
    match (acc, value) {
        (Some(a), Some(v)) => Some(a.max(v)),
        (None, v) => v,
        (a, None) => a,
    }
}

/// All-time maxima over the seeded marks, every history row, and the current
/// values. Rate and gain are tracked independently and may peak at different
/// moments. `current` is optional: the seeded marks and history alone still
/// define a max when no current values exist.
pub fn mpf_max(
    history: &[MpfPoint],
    current: Option<MpfPoint>,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
) -> MpfFigures {
    let mut rate = seed_max_rate;
    let mut gain = seed_max_gain;
    for point in history.iter().copied().chain(current) {
        let figures = mpf_figures(point.contributions, point.balance);
        rate = fold_max(rate, figures.rate);
        gain = fold_max(gain, Some(figures.gain));
    }
    MpfFigures {
        rate,
        gain: gain.unwrap_or(0.0),
    }
}

/// The account's standing values on `date`: the latest history row on or
/// before it; otherwise the current values when the record already existed,
/// else nothing.
fn mpf_as_of(facts: &MpfAccountFacts, date: chrono::NaiveDate) -> Option<MpfPoint> {
    if let Some(point) = facts
        .history
        .iter()
        .filter(|point| point.recorded_on <= date)
        .max_by_key(|point| point.recorded_on)
    {
        return Some(*point);
    }
    (facts.created_on <= date).then_some(MpfPoint {
        recorded_on: date,
        contributions: facts.contributions,
        balance: facts.balance,
    })
}

/// Section aggregates via an as-of merge: at every recorded date (plus last
/// month-end and today) each account contributes its standing values. This
/// carries an untouched account forward and gives a true portfolio max rather
/// than summing per-account peaks that may never have co-occurred. The
/// portfolio seeds floor the maxima.
pub fn mpf_totals(
    accounts: &[MpfAccountFacts],
    today: chrono::NaiveDate,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
) -> MpfTotals {
    // The standing values count as a point at `today`, so an account whose
    // today's history row was deleted (or never written) still reports its
    // current state rather than a stale row.
    let extended: Vec<MpfAccountFacts> = accounts
        .iter()
        .map(|account| {
            let mut facts = account.clone();
            facts.history.push(MpfPoint {
                recorded_on: today,
                contributions: account.contributions,
                balance: account.balance,
            });
            facts
        })
        .collect();

    let prev_end = first_of_month(today) - chrono::Days::new(1);
    let mut dates: Vec<chrono::NaiveDate> = extended
        .iter()
        .flat_map(|account| account.history.iter().map(|point| point.recorded_on))
        .collect();
    dates.push(prev_end);
    dates.sort_unstable();
    dates.dedup();

    let mut max_rate = seed_max_rate;
    let mut max_gain = seed_max_gain;
    let mut last_month = None;
    for date in dates {
        let (mut contributions, mut balance) = (0.0, 0.0);
        let mut any = false;
        for account in &extended {
            if let Some(point) = mpf_as_of(account, date) {
                contributions += point.contributions;
                balance += point.balance;
                any = true;
            }
        }
        if !any {
            continue;
        }
        let figures = mpf_figures(contributions, balance);
        if date == prev_end {
            last_month = Some(figures);
        }
        max_rate = fold_max(max_rate, figures.rate);
        max_gain = fold_max(max_gain, Some(figures.gain));
    }

    let buy: f64 = accounts.iter().map(|account| account.contributions).sum();
    let now: f64 = accounts.iter().map(|account| account.balance).sum();
    let current = mpf_figures(buy, now);
    MpfTotals {
        buy,
        now,
        rate: current.rate,
        gain: current.gain,
        last_month,
        max: MpfFigures {
            rate: max_rate,
            gain: max_gain.unwrap_or(current.gain),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

    fn mpf_point(date: &str, contributions: f64, balance: f64) -> MpfPoint {
        MpfPoint {
            recorded_on: day(date),
            contributions,
            balance,
        }
    }

    #[test]
    fn mpf_rate_is_gain_over_contributions() {
        let figures = mpf_figures(100_000.0, 125_000.0);
        assert!(approx_eq(figures.rate.unwrap(), 0.25));
        assert!(approx_eq(figures.gain, 25_000.0));
    }

    #[test]
    fn mpf_zero_contributions_leave_rate_empty() {
        let figures = mpf_figures(0.0, 500.0);
        assert!(figures.rate.is_none());
        assert!(approx_eq(figures.gain, 500.0));
    }

    #[test]
    fn mpf_last_month_reads_the_latest_row_in_the_previous_month() {
        let history = [
            mpf_point("2026-08-10", 100.0, 100.0),
            mpf_point("2026-08-30", 100.0, 112.0),
            mpf_point("2026-09-05", 100.0, 115.0),
        ];
        let last = mpf_last_month(&history, day("2026-09-17")).unwrap();
        assert!(approx_eq(last.rate.unwrap(), 0.12));
        assert!(approx_eq(last.gain, 12.0));
        // Only a September row: August has nothing to report.
        assert!(mpf_last_month(&history[2..], day("2026-09-17")).is_none());
    }

    #[test]
    fn mpf_last_month_carries_forward_older_rows() {
        // No row in September: the August row still stood at September's end.
        let history = [mpf_point("2026-08-28", 100.0, 115.0)];
        let last = mpf_last_month(&history, day("2026-10-05")).unwrap();
        assert!(approx_eq(last.rate.unwrap(), 0.15));
        assert!(approx_eq(last.gain, 15.0));
        // Rows only in the current month never count.
        let current = [mpf_point("2026-10-02", 100.0, 115.0)];
        assert!(mpf_last_month(&current, day("2026-10-05")).is_none());
    }

    #[test]
    fn mpf_max_tracks_rate_and_gain_independently() {
        let history = [
            mpf_point("2026-03-10", 100.0, 148.0), // rate 48%, gain 48
            mpf_point("2026-08-10", 120.0, 180.0), // rate 50%, gain 60
        ];
        let current = mpf_point("2026-10-01", 200.0, 300.0); // rate 50%, gain 100
        let max = mpf_max(&history, Some(current), None, None);
        assert!(approx_eq(max.rate.unwrap(), 0.5));
        assert!(approx_eq(max.gain, 100.0));
    }

    #[test]
    fn mpf_max_is_floored_by_the_seeded_marks() {
        let current = mpf_point("2026-09-17", 100.0, 110.0);
        let max = mpf_max(&[], Some(current), Some(0.4792), Some(278_899.91));
        assert!(approx_eq(max.rate.unwrap(), 0.4792));
        assert!(approx_eq(max.gain, 278_899.91));
    }

    #[test]
    fn mpf_totals_carry_an_untouched_account_forward() {
        // Account A updated in Aug and Oct; B only ever recorded in Aug.
        let a = MpfAccountFacts {
            created_on: day("2026-08-01"),
            contributions: 100.0,
            balance: 150.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![
                mpf_point("2026-08-28", 100.0, 110.0),
                mpf_point("2026-10-05", 100.0, 150.0),
            ],
        };
        let b = MpfAccountFacts {
            created_on: day("2026-08-01"),
            contributions: 200.0,
            balance: 260.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![mpf_point("2026-08-28", 200.0, 240.0)],
        };
        let totals = mpf_totals(&[a, b], day("2026-10-17"), None, None);

        // Current: buy 300, now 410, rate 110/300.
        assert!(approx_eq(totals.buy, 300.0));
        assert!(approx_eq(totals.now, 410.0));
        assert!(approx_eq(totals.rate.unwrap(), 110.0 / 300.0));
        // Last month-end (Sep 30): A as-of = Aug 28 row, B = its Aug row —
        // carried forward even though neither updated in September.
        let last = totals.last_month.unwrap();
        assert!(approx_eq(last.rate.unwrap(), 50.0 / 300.0));
        // Both maxima sit at today: gain 110, rate 110/300.
        assert!(approx_eq(totals.max.gain, 110.0));
        assert!(approx_eq(totals.max.rate.unwrap(), 110.0 / 300.0));
    }

    #[test]
    fn mpf_totals_exclude_an_account_before_it_existed() {
        let old = MpfAccountFacts {
            created_on: day("2026-01-01"),
            contributions: 100.0,
            balance: 150.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![mpf_point("2026-01-31", 100.0, 120.0)],
        };
        let new = MpfAccountFacts {
            created_on: day("2026-09-20"),
            contributions: 50.0,
            balance: 50.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![],
        };
        let totals = mpf_totals(&[old, new], day("2026-10-17"), None, None);
        // At Jan 31 only `old` existed: rate 0.2. Had `new`'s current values
        // leaked into that date the portfolio rate would have been 0.4667 and
        // become the max — instead the max is today's 50/150.
        assert!(approx_eq(totals.max.rate.unwrap(), 50.0 / 150.0));
        assert!(approx_eq(totals.max.gain, 50.0));
        // Last month-end (Sep 30): `new` existed (created Sep 20) with no
        // rows yet, so its current values stand in; `old` carries Jan's row.
        let last = totals.last_month.unwrap();
        assert!(approx_eq(last.rate.unwrap(), 20.0 / 150.0));
        assert!(approx_eq(last.gain, 20.0));
    }
}
