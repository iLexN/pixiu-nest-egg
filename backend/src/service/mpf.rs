use crate::calc::{MpfPoint, mpf_figures, mpf_last_month, mpf_max};
use crate::models::{MpfAccount, MpfHistoryRow};
use crate::store::mpf::{self, StoredAccount};

/// A stored MPF account plus its derived figures and history cells.
pub fn present_account(
    account: &StoredAccount,
    history: &[MpfHistoryRow],
    today: chrono::NaiveDate,
) -> MpfAccount {
    let facts = mpf::facts_of(account, history);
    let current = MpfPoint {
        recorded_on: today,
        contributions: account.contributions,
        balance: account.balance,
    };
    let figures = mpf_figures(account.contributions, account.balance);
    MpfAccount {
        id: account.id,
        label: account.label.clone(),
        trustee: account.trustee.clone(),
        contributions: account.contributions,
        balance: account.balance,
        plan_name: account.plan_name.clone(),
        member_no: account.member_no.clone(),
        sort_order: account.sort_order,
        rate: figures.rate,
        gain: figures.gain,
        last_month: mpf_last_month(&facts.history, today),
        max: mpf_max(
            &facts.history,
            Some(current),
            account.seed_max_rate,
            account.seed_max_gain,
        ),
    }
}
