# Proposal

## Why

Completes candidate 4 of the architecture review ("give the live-wealth aggregate its own module"): `live_totals_input` already moved to `service::wealth` with month-book, but the settings defaults it shares a home with — `goal_constants` and `semi_liquid_target` plus their `DEFAULT_*` literals — still lived in `service::months`, an overview/forecast concept asked of the month book.

## What Changes

`service::months::{goal_constants, semi_liquid_target, DEFAULT_GOAL3_INFLATION, DEFAULT_GOAL3_YEARS, DEFAULT_FINAL_REPLACEMENT, DEFAULT_SEMI_LIQUID_TARGET}` → `service::settings`; 4 call sites repointed. Pure move, no behavior change.

## Capabilities

None — pure refactor; `skip_specs: true`.

## Impact

`backend/src/service/{months,settings,overview}.rs`, `backend/src/routes/{months,forecast}.rs`.
