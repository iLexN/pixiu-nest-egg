//! The `/api/settings` get/update: the scalar `app_meta` settings plus
//! the current year's pool rate, with every field's validation.

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{AppSettings, AppSettingsPatch};
use crate::service::ServiceError;
use crate::service::months::{
    DEFAULT_FINAL_REPLACEMENT, DEFAULT_GOAL3_INFLATION, DEFAULT_GOAL3_YEARS,
    DEFAULT_SEMI_LIQUID_TARGET,
};
use crate::store::months::{self, DEFAULT_BILL_AMOUNT};
use crate::store::{StoreError, settings, today};

/// The stored settings plus the effective defaults — the same projection the
/// settings endpoint returns.
pub async fn get(pool: &SqlitePool) -> Result<AppSettings, StoreError> {
    let stored = settings::Settings::load(pool).await?;
    let pool_rate_year = today().year();
    Ok(AppSettings {
        salary: stored.salary,
        pool_rate: months::pool_rate(pool, pool_rate_year).await?,
        pool_rate_year,
        bill_amount: stored.bill_amount.unwrap_or(DEFAULT_BILL_AMOUNT),
        semi_liquid_target: stored
            .semi_liquid_target
            .unwrap_or(DEFAULT_SEMI_LIQUID_TARGET),
        money_master_start_date: stored.money_master_start_date,
        money_master_saved: stored.money_master_saved,
        money_master_target_months: stored.money_master_target_months,
        money_master_target_amount: stored.money_master_target_amount,
        money_master_month_now: stored.money_master_month_now,
        money_master_coming_save: stored.money_master_coming_save,
        goal1_amount: stored.goal1_amount,
        goal2_amount: stored.goal2_amount,
        goal3_inflation: stored.goal3_inflation.unwrap_or(DEFAULT_GOAL3_INFLATION),
        goal3_years: stored.goal3_years.unwrap_or(DEFAULT_GOAL3_YEARS),
        final_replacement: stored
            .final_replacement
            .unwrap_or(DEFAULT_FINAL_REPLACEMENT),
    })
}

/// Apply a settings patch after validating every supplied field; `null`
/// clears a key (the effective value falls back to its default).
pub async fn update(
    pool: &SqlitePool,
    patch: &AppSettingsPatch,
) -> Result<AppSettings, ServiceError> {
    let mut errors = Vec::new();
    if let Some(Some(salary)) = patch.salary
        && (!salary.is_finite() || salary < 0.0)
    {
        errors.push(FieldError::new("salary", "salary must not be negative"));
    }
    if let Some(Some(rate)) = patch.pool_rate
        && (!rate.is_finite() || !(0.0..1.0).contains(&rate))
    {
        errors.push(FieldError::new(
            "pool_rate",
            "pool rate is stored as a fraction (0.337 = 33.7%) and must be less than 1",
        ));
    }
    if let Some(Some(amount)) = patch.bill_amount
        && (!amount.is_finite() || amount < 0.0)
    {
        errors.push(FieldError::new(
            "bill_amount",
            "bill amount must not be negative",
        ));
    }
    if let Some(Some(ratio)) = patch.semi_liquid_target
        && (!ratio.is_finite() || !(0.0..1.0).contains(&ratio))
    {
        errors.push(FieldError::new(
            "semi_liquid_target",
            "semi-liquid target is stored as a fraction (0.25 = 25%) and must be less than 1",
        ));
    }
    if let Some(Some(date)) = &patch.money_master_start_date
        && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
    {
        errors.push(FieldError::new(
            "money_master_start_date",
            "date must be YYYY-MM-DD",
        ));
    }
    for (field, value) in [
        ("money_master_month_now", patch.money_master_month_now),
        (
            "money_master_target_months",
            patch.money_master_target_months,
        ),
    ] {
        if let Some(Some(value)) = value
            && (!value.is_finite() || value <= 0.0 || value.fract() != 0.0)
        {
            errors.push(FieldError::new(field, "must be a positive whole number"));
        }
    }
    if let Some(Some(amount)) = patch.money_master_target_amount
        && (!amount.is_finite() || amount <= 0.0)
    {
        errors.push(FieldError::new(
            "money_master_target_amount",
            "must be positive",
        ));
    }
    // The bank's saved figure and coming-save are sign-free — coming save
    // legitimately goes negative while the challenge runs ahead of target.
    for (field, value) in [
        ("money_master_saved", patch.money_master_saved),
        ("money_master_coming_save", patch.money_master_coming_save),
    ] {
        if let Some(Some(value)) = value
            && !value.is_finite()
        {
            errors.push(FieldError::new(field, "must be a number"));
        }
    }
    for (field, value) in [
        ("goal1_amount", patch.goal1_amount),
        ("goal2_amount", patch.goal2_amount),
    ] {
        if let Some(Some(value)) = value
            && (!value.is_finite() || value < 0.0)
        {
            errors.push(FieldError::new(field, "goal amount must not be negative"));
        }
    }
    for (field, value) in [
        ("goal3_inflation", patch.goal3_inflation),
        ("goal3_years", patch.goal3_years),
        ("final_replacement", patch.final_replacement),
    ] {
        if let Some(Some(value)) = value
            && (!value.is_finite() || value <= 0.0)
        {
            errors.push(FieldError::new(field, "must be positive"));
        }
    }
    if !errors.is_empty() {
        return Err(ServiceError::Validation(errors));
    }

    let settings_patch = settings::SettingsPatch {
        salary: patch.salary,
        bill_amount: patch.bill_amount,
        semi_liquid_target: patch.semi_liquid_target,
        goal1_amount: patch.goal1_amount,
        goal2_amount: patch.goal2_amount,
        goal3_inflation: patch.goal3_inflation,
        goal3_years: patch.goal3_years,
        final_replacement: patch.final_replacement,
        money_master_start_date: patch.money_master_start_date.clone(),
        money_master_saved: patch.money_master_saved,
        money_master_target_months: patch.money_master_target_months,
        money_master_target_amount: patch.money_master_target_amount,
        money_master_month_now: patch.money_master_month_now,
        money_master_coming_save: patch.money_master_coming_save,
        ..Default::default()
    };
    let mut conn = pool.acquire().await?;
    settings_patch.apply(&mut conn).await?;
    drop(conn);
    if let Some(rate) = patch.pool_rate {
        let year = patch.pool_rate_year.unwrap_or_else(|| today().year());
        months::set_pool_rate(pool, year, rate).await?;
    }
    get(pool).await.map_err(ServiceError::from)
}
