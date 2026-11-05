# Glossary

Domain terms as this codebase uses them. Code, specs, docs, and proposals
should use these names rather than synonyms.

## Month book

The assembled 月結 ledger for reads: all stored `month_stats` rows, the
per-month `month_items`, the derived stat rows, the live totals (computed only
when a stored NULL or the current-month tail needs them), and the live tail
(the live figures the last row's Changed cells diff against — present only
when the last stored month IS the current month). Served by
`service::months::MonthBook` — `MonthBook::open(pool)` computes live behind
the gate; `open_with_live(pool, totals)` takes totals the caller already
holds. Every month-derived read (list, summary, detail, forecast, overview,
year-review) opens the book rather than sequencing loads itself.

## Live totals

The 總資產/流動資產 aggregate — the sheet's `Overview!B1`/`H1` computed from
current data: HK + US stock market value, active 定期/債券 principal, AIA,
MPF, manual assets, 活期, IBKR cash, minus/plus 開心Pool. Computed by
`service::wealth::totals_input` → `calc::live_totals`. "Live" because it
reflects now, not a stored figure; month NULL totals fill from it at/after
the current month.
