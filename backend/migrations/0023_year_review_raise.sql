-- 投資目標 (Overview J22:N27): the year's monthly salary increment (月薪增幅).
-- NULL derives live as last salary(Y) − last salary(Y-1) from month rows; a
-- stored value overrides. Negative is allowed — a deliberate target cut.
-- Named salary_raise: `raise` is a SQLite keyword (the RAISE() trigger fn).
ALTER TABLE year_review ADD COLUMN salary_raise REAL;
