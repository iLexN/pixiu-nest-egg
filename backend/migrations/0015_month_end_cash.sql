-- A hand-frozen H 月尾 value: the `=F(n+1) − salary` chain cannot reproduce
-- months whose sheet cell was typed by hand (2023-12 stored the real bank
-- balance because no next-row 月初 convention existed yet). NULL = derive.
ALTER TABLE month_stats ADD COLUMN end_cash_override REAL;
