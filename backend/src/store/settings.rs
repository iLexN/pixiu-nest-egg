//! Every `app_meta` key lives here — the typed settings, the free-text notes,
//! and the seed marks. `store::meta` is private to this module.

use sqlx::SqlitePool;

use crate::models::Market;
use crate::store::StoreError;
use crate::store::meta::{keys_like, meta_f64, meta_get, meta_put};

const SALARY_KEY: &str = "ledger.salary";
const BILL_AMOUNT_KEY: &str = "forecast.bill_amount";
const SEMI_LIQUID_TARGET_KEY: &str = "strategy.semi_liquid_target";
const GOAL1_AMOUNT_KEY: &str = "goals.goal1_amount";
const GOAL2_AMOUNT_KEY: &str = "goals.goal2_amount";
const GOAL3_INFLATION_KEY: &str = "goals.goal3_inflation";
const GOAL3_YEARS_KEY: &str = "goals.goal3_years";
const FINAL_REPLACEMENT_KEY: &str = "goals.final_replacement";
const MM_START_DATE_KEY: &str = "money_master.start_date";
const MM_SAVED_KEY: &str = "money_master.saved";
const MM_TARGET_MONTHS_KEY: &str = "money_master.target_months";
const MM_TARGET_AMOUNT_KEY: &str = "money_master.target_amount";
const MM_MONTH_NOW_KEY: &str = "money_master.month_now";
const MM_COMING_SAVE_KEY: &str = "money_master.coming_save";
const IBKR_REPORTED_VALUE_KEY: &str = "ibkr.reported_value";
const AIA_NOTE_KEY: &str = "aia.note";
const MPF_NOTE_KEY: &str = "mpf.note";
const MPF_SEED_MAX_RATE_KEY: &str = "mpf.seed_max_rate";
const MPF_SEED_MAX_GAIN_KEY: &str = "mpf.seed_max_gain";
const FAMILY_NOTE_PREFIX: &str = "family.note.";

fn market_seed_max_percent_key(market: Market) -> String {
    format!("market.{}.seed_max_percent", market.as_str())
}

fn market_seed_max_amount_key(market: Market) -> String {
    format!("market.{}.seed_max_amount", market.as_str())
}

/// The stored scalar settings — `Option` fields are `None` while the key is
/// unset; effective defaults live where they are applied.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub salary: Option<f64>,
    pub bill_amount: Option<f64>,
    pub semi_liquid_target: Option<f64>,
    pub goal1_amount: Option<f64>,
    pub goal2_amount: Option<f64>,
    pub goal3_inflation: Option<f64>,
    pub goal3_years: Option<f64>,
    pub final_replacement: Option<f64>,
    pub money_master_start_date: Option<String>,
    pub money_master_saved: Option<f64>,
    pub money_master_target_months: Option<f64>,
    pub money_master_target_amount: Option<f64>,
    pub money_master_month_now: Option<f64>,
    pub money_master_coming_save: Option<f64>,
    pub ibkr_reported_value: Option<f64>,
}

impl Settings {
    pub async fn load(pool: &SqlitePool) -> Result<Settings, StoreError> {
        Ok(Settings {
            salary: meta_f64(pool, SALARY_KEY).await?,
            bill_amount: meta_f64(pool, BILL_AMOUNT_KEY).await?,
            semi_liquid_target: meta_f64(pool, SEMI_LIQUID_TARGET_KEY).await?,
            goal1_amount: meta_f64(pool, GOAL1_AMOUNT_KEY).await?,
            goal2_amount: meta_f64(pool, GOAL2_AMOUNT_KEY).await?,
            goal3_inflation: meta_f64(pool, GOAL3_INFLATION_KEY).await?,
            goal3_years: meta_f64(pool, GOAL3_YEARS_KEY).await?,
            final_replacement: meta_f64(pool, FINAL_REPLACEMENT_KEY).await?,
            money_master_start_date: meta_get(pool, MM_START_DATE_KEY).await?,
            money_master_saved: meta_f64(pool, MM_SAVED_KEY).await?,
            money_master_target_months: meta_f64(pool, MM_TARGET_MONTHS_KEY).await?,
            money_master_target_amount: meta_f64(pool, MM_TARGET_AMOUNT_KEY).await?,
            money_master_month_now: meta_f64(pool, MM_MONTH_NOW_KEY).await?,
            money_master_coming_save: meta_f64(pool, MM_COMING_SAVE_KEY).await?,
            ibkr_reported_value: meta_f64(pool, IBKR_REPORTED_VALUE_KEY).await?,
        })
    }
}

/// A partial settings write: `None` leaves the key untouched, `Some(None)`
/// deletes it.
#[derive(Debug, Clone, Default)]
pub struct SettingsPatch {
    pub salary: Option<Option<f64>>,
    pub bill_amount: Option<Option<f64>>,
    pub semi_liquid_target: Option<Option<f64>>,
    pub goal1_amount: Option<Option<f64>>,
    pub goal2_amount: Option<Option<f64>>,
    pub goal3_inflation: Option<Option<f64>>,
    pub goal3_years: Option<Option<f64>>,
    pub final_replacement: Option<Option<f64>>,
    pub money_master_start_date: Option<Option<String>>,
    pub money_master_saved: Option<Option<f64>>,
    pub money_master_target_months: Option<Option<f64>>,
    pub money_master_target_amount: Option<Option<f64>>,
    pub money_master_month_now: Option<Option<f64>>,
    pub money_master_coming_save: Option<Option<f64>>,
    pub ibkr_reported_value: Option<Option<f64>>,
}

impl SettingsPatch {
    /// `conn` may be a transaction — `apply` is composed into callers'
    /// transactions.
    pub async fn apply(&self, conn: &mut sqlx::SqliteConnection) -> Result<(), StoreError> {
        for (key, value) in [
            (SALARY_KEY, self.salary),
            (BILL_AMOUNT_KEY, self.bill_amount),
            (SEMI_LIQUID_TARGET_KEY, self.semi_liquid_target),
            (GOAL1_AMOUNT_KEY, self.goal1_amount),
            (GOAL2_AMOUNT_KEY, self.goal2_amount),
            (GOAL3_INFLATION_KEY, self.goal3_inflation),
            (GOAL3_YEARS_KEY, self.goal3_years),
            (FINAL_REPLACEMENT_KEY, self.final_replacement),
            (MM_SAVED_KEY, self.money_master_saved),
            (MM_TARGET_MONTHS_KEY, self.money_master_target_months),
            (MM_TARGET_AMOUNT_KEY, self.money_master_target_amount),
            (MM_MONTH_NOW_KEY, self.money_master_month_now),
            (MM_COMING_SAVE_KEY, self.money_master_coming_save),
            (IBKR_REPORTED_VALUE_KEY, self.ibkr_reported_value),
        ] {
            if let Some(value) = value {
                meta_put(&mut *conn, key, value.map(|v| v.to_string()).as_deref()).await?;
            }
        }
        if let Some(date) = &self.money_master_start_date {
            meta_put(&mut *conn, MM_START_DATE_KEY, date.as_deref()).await?;
        }
        Ok(())
    }
}

/// The AIA page's free-text note; an empty note deletes the key.
pub async fn aia_note(pool: &SqlitePool) -> Result<Option<String>, StoreError> {
    meta_get(pool, AIA_NOTE_KEY).await
}

pub async fn set_aia_note(pool: &SqlitePool, note: Option<&str>) -> Result<(), StoreError> {
    meta_put(pool, AIA_NOTE_KEY, note).await
}

/// The MPF page's free-text note.
pub async fn mpf_note(pool: &SqlitePool) -> Result<Option<String>, StoreError> {
    meta_get(pool, MPF_NOTE_KEY).await
}

pub async fn set_mpf_note(pool: &SqlitePool, note: Option<&str>) -> Result<(), StoreError> {
    meta_put(pool, MPF_NOTE_KEY, note).await
}

/// The per-holder 家人 定期 note: `family.note.<holder>`.
pub async fn family_note(pool: &SqlitePool, holder: &str) -> Result<Option<String>, StoreError> {
    meta_get(pool, &format!("{FAMILY_NOTE_PREFIX}{holder}")).await
}

pub async fn set_family_note(
    pool: &SqlitePool,
    holder: &str,
    note: Option<&str>,
) -> Result<(), StoreError> {
    meta_put(pool, &format!("{FAMILY_NOTE_PREFIX}{holder}"), note).await
}

/// Holders with a stored `family.note.<holder>` key, sorted.
pub async fn family_note_holders(pool: &SqlitePool) -> Result<Vec<String>, StoreError> {
    Ok(keys_like(pool, "family.note.%")
        .await?
        .into_iter()
        .filter_map(|key| key.strip_prefix(FAMILY_NOTE_PREFIX).map(str::to_string))
        .collect())
}

/// The MPF portfolio's stored all-time marks: (`seed_max_rate`, `seed_max_gain`).
pub async fn mpf_seed_max(pool: &SqlitePool) -> Result<(Option<f64>, Option<f64>), StoreError> {
    Ok((
        meta_f64(pool, MPF_SEED_MAX_RATE_KEY).await?,
        meta_f64(pool, MPF_SEED_MAX_GAIN_KEY).await?,
    ))
}

/// The workbook's seeded per-market maxima: (`seed_max_percent`, `seed_max_amount`).
pub async fn market_seed_max(
    pool: &SqlitePool,
    market: Market,
) -> Result<(Option<f64>, Option<f64>), StoreError> {
    Ok((
        meta_f64(pool, &market_seed_max_percent_key(market)).await?,
        meta_f64(pool, &market_seed_max_amount_key(market)).await?,
    ))
}
