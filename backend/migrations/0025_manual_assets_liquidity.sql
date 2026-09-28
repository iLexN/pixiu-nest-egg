-- 策略 (Overview J3:K7): whether a manual `asset` row is recoverable
-- short- or long-term. `short` rows count in K6 短期可取回, `long` in K7
-- 長期可取回. The column is NOT NULL so `cash` rows carry it too, but it has
-- no effect on them — cash already sits inside 半流動資金.
ALTER TABLE manual_assets ADD COLUMN liquidity TEXT NOT NULL DEFAULT 'long'
    CHECK (liquidity IN ('short','long'));
