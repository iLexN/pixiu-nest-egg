//! Read-only reader for the spreadsheet being replaced.
//!
//! Only the trade tables and the per-stock metadata/summary columns are read;
//! everything else on those sheets (the 派息 table, templates, scratch cells)
//! is ignored. Formula cells are read as their cached values.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context};
use calamine::{open_workbook_auto, Data, Reader};

use crate::models::{InputMode, Market};

pub const HK_TRADE_SHEET: &str = "港股Trade";
pub const HK_SUMMARY_SHEET: &str = "港股";
pub const US_TRADE_SHEET: &str = "美股Trade";
pub const US_SUMMARY_SHEET: &str = "美股";
pub const DEPOSIT_SHEET: &str = "定期";
pub const DEPOSIT_INFO_SHEET: &str = "定期Info";

#[derive(Debug, Clone)]
pub struct SheetTrade {
    pub code: String,
    pub shares: f64,
    pub unit_price: f64,
    pub total: Option<f64>,
    pub fee: Option<f64>,
    pub trade_date: String,
    pub trade_type: String,
    pub input_mode: InputMode,
    pub note: Option<String>,
    /// 1-based row number in the source sheet, for error messages.
    pub source_row: usize,
}

#[derive(Debug, Clone)]
pub struct SheetStock {
    pub code: String,
    pub ticker: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    /// 1-based row order on the market's summary sheet.
    pub sort_order: i64,
}

/// Cached per-stock figures from the summary sheet, used by the parity check.
#[derive(Debug, Clone)]
pub struct SheetSummary {
    pub code: String,
    pub shares_held: Option<f64>,
    pub weighted_avg_buy_price: Option<f64>,
    pub total_buy_cost: Option<f64>,
}

/// One row of the trade sheet's J–O 派息 block.
#[derive(Debug, Clone)]
pub struct SheetDividend {
    /// J: the stock name/code.
    pub code: String,
    /// K: the pay date.
    pub pay_date: String,
    /// M: 派息 amount (cached value).
    pub amount: f64,
    /// L cached rate: amount ÷ 總買入成本 snapshot.
    pub rate_on_cost: Option<f64>,
    /// N cached rate: amount ÷ (price × 股數).
    pub rate_on_price: Option<f64>,
    /// O: 股數 snapshot.
    pub shares: Option<f64>,
    /// The M cell's formula text, when it is a formula.
    pub amount_formula: Option<String>,
    /// 1-based row number in the source sheet, for error messages.
    pub source_row: usize,
}

#[derive(Debug, Clone)]
pub struct MarketSheets {
    pub market: Market,
    pub trades: Vec<SheetTrade>,
    pub stocks: Vec<SheetStock>,
    pub summary: Vec<SheetSummary>,
    pub dividends: Vec<SheetDividend>,
}

/// One row of 定期Info's 表_定期List. The sheet's derived columns
/// (status, total, month, year) are not read; the app re-derives them.
#[derive(Debug, Clone)]
pub struct SheetDeposit {
    /// The sheet's `id` column: a bank reference like `SC-9632`.
    pub label: Option<String>,
    /// The sheet's `input` column.
    pub principal: Option<f64>,
    /// Annual rate as a fraction (0.03 = 3%).
    pub rate: Option<f64>,
    /// 利息.
    pub interest: Option<f64>,
    pub end_date: String,
    pub note1: Option<String>,
    pub note2: Option<String>,
    /// 1-based position within the table, stored as sort_order.
    pub sort_order: i64,
    /// 1-based row number in the source sheet, for error messages.
    pub source_row: usize,
}

/// A 定期 month-table or bank row: B Total (Σ deposit total), C 利息,
/// D 定期 (Σ principal).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetActiveSums {
    pub total: f64,
    pub interest: f64,
    pub principal: f64,
}

/// A 定期Info year-table month row: B Total (= 利息 + 定期), C 利息,
/// D 定期 (Σ deposit total).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetYearSums {
    pub total: f64,
    pub interest: f64,
    pub payout: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetYearTable {
    pub year: i32,
    /// (month number, sums) for each month the table lists.
    pub months: Vec<(u32, SheetYearSums)>,
}

/// The workbook's cached deposit aggregates, for the parity check.
#[derive(Debug, Clone, Default)]
pub struct DepositSheetCached {
    /// 定期!B1: Σ active principal.
    pub active_principal: Option<f64>,
    /// The 定期 month table: month number -> sums.
    pub months: Vec<(u32, SheetActiveSums)>,
    /// The month table's `total` row, when present.
    pub grand_total: Option<SheetActiveSums>,
    /// The 定期 bank rows keyed by the label in column A (SC, HS).
    pub banks: Vec<(String, SheetActiveSums)>,
    /// The 定期Info year tables.
    pub years: Vec<SheetYearTable>,
}

#[derive(Debug, Clone)]
pub struct WorkbookData {
    pub hk: MarketSheets,
    pub us: MarketSheets,
    pub deposits: Vec<SheetDeposit>,
    pub deposit_cached: DepositSheetCached,
}

impl WorkbookData {
    pub fn market(&self, market: Market) -> &MarketSheets {
        match market {
            Market::Hk => &self.hk,
            Market::Us => &self.us,
        }
    }
}

type Rows = Vec<Vec<Data>>;

/// Open the workbook read-only and extract both markets.
pub fn read(path: &Path) -> anyhow::Result<WorkbookData> {
    if !path.exists() {
        return Err(anyhow!("workbook {} does not exist", path.display()));
    }
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("cannot open workbook {}", path.display()))?;

    let hk_trades = sheet_rows(&mut workbook, HK_TRADE_SHEET, path)?;
    let hk_summary = sheet_rows(&mut workbook, HK_SUMMARY_SHEET, path)?;
    let us_trades = sheet_rows(&mut workbook, US_TRADE_SHEET, path)?;
    let us_summary = sheet_rows(&mut workbook, US_SUMMARY_SHEET, path)?;
    let deposit_info = sheet_rows(&mut workbook, DEPOSIT_INFO_SHEET, path)?;
    let deposit_view = sheet_rows(&mut workbook, DEPOSIT_SHEET, path)?;
    // Formula text is a second pass; formats without formulas error, which is
    // fine — the note just stays empty then.
    let hk_formulas = workbook.worksheet_formula(HK_TRADE_SHEET).ok();
    let us_formulas = workbook.worksheet_formula(US_TRADE_SHEET).ok();

    Ok(WorkbookData {
        hk: MarketSheets {
            market: Market::Hk,
            trades: parse_trades(&hk_trades, Market::Hk)?,
            stocks: parse_hk_stocks(&hk_summary),
            summary: parse_summary(&hk_summary, Market::Hk),
            dividends: parse_dividends(&hk_trades, hk_formulas.as_ref()),
        },
        us: MarketSheets {
            market: Market::Us,
            trades: parse_trades(&us_trades, Market::Us)?,
            stocks: parse_us_stocks(&us_summary),
            summary: parse_summary(&us_summary, Market::Us),
            dividends: parse_dividends(&us_trades, us_formulas.as_ref()),
        },
        deposits: parse_deposits(&deposit_info)?,
        deposit_cached: parse_deposit_cached(&deposit_view, &deposit_info),
    })
}

fn sheet_rows<R>(workbook: &mut R, name: &str, path: &Path) -> anyhow::Result<Rows>
where
    R: Reader<std::io::BufReader<std::fs::File>>,
    R::Error: std::fmt::Display,
{
    let range = workbook.worksheet_range(name).map_err(|err| {
        anyhow!(
            "workbook {} has no usable sheet named {name}: {err}",
            path.display()
        )
    })?;
    Ok(range.rows().map(<[Data]>::to_vec).collect())
}

fn cell(row: &[Data], index: usize) -> Option<&Data> {
    row.get(index).filter(|value| **value != Data::Empty)
}

fn number(row: &[Data], index: usize) -> Option<f64> {
    match cell(row, index)? {
        Data::Float(value) => Some(*value),
        Data::Int(value) => Some(*value as f64),
        Data::String(value) => value.trim().parse::<f64>().ok(),
        Data::DateTime(value) => Some(value.as_f64()),
        _ => None,
    }
}

fn text(row: &[Data], index: usize) -> Option<String> {
    let value = match cell(row, index)? {
        Data::String(value) => value.trim().to_string(),
        // Tickers such as 3988 come back as numbers; keep them digit-only.
        Data::Float(value) if value.fract() == 0.0 => format!("{}", *value as i64),
        Data::Float(value) => format!("{value}"),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        _ => return None,
    };
    Some(value).filter(|v| !v.is_empty())
}

fn date(row: &[Data], index: usize) -> Option<String> {
    match cell(row, index)? {
        Data::DateTime(value) => value.as_datetime().map(|dt| dt.date().to_string()),
        Data::DateTimeIso(value) => value.get(..10).map(str::to_string),
        Data::String(value) => {
            let value = value.trim();
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .map(|d| d.to_string())
        }
        _ => None,
    }
}

/// A date cell that may hold a raw Excel serial (days since 1899-12-30), as
/// the 派息 block's K column does.
fn serial_or_date(row: &[Data], index: usize) -> Option<String> {
    date(row, index).or_else(|| {
        let serial = number(row, index)?;
        // Plausible serial range: 1954-10 through 2173-10.
        if !(20000.0..=100000.0).contains(&serial) {
            return None;
        }
        let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
        Some(
            (epoch + chrono::Duration::days(serial.trunc() as i64))
                .format("%Y-%m-%d")
                .to_string(),
        )
    })
}

// Trade sheet columns J–O hold the 派息 block: J stock, K pay date, L rate
// (M ÷ buy cost), M 派息 amount, N second rate (M ÷ price × shares), O 股數.
const DIVIDEND_STOCK: usize = 9;
const DIVIDEND_DATE: usize = 10;
const DIVIDEND_RATE_COST: usize = 11;
const DIVIDEND_AMOUNT: usize = 12;
const DIVIDEND_RATE_PRICE: usize = 13;
const DIVIDEND_SHARES: usize = 14;

/// Rows where J (stock), K (date) and M (amount) are all populated form the
/// 派息 block; everything else — the trade table's own columns, scratch cells,
/// the header row — falls out naturally.
fn parse_dividends(rows: &Rows, formulas: Option<&calamine::Range<String>>) -> Vec<SheetDividend> {
    let mut dividends = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let (Some(code), Some(pay_date), Some(amount)) = (
            text(row, DIVIDEND_STOCK),
            serial_or_date(row, DIVIDEND_DATE),
            number(row, DIVIDEND_AMOUNT),
        ) else {
            continue;
        };
        // calamine's Range::get indexes relative to the formula range's
        // start, which is not A1.
        let amount_formula = formulas.and_then(|range| {
            let (start_row, start_col) = range.start().unwrap_or((0, 0));
            let (start_row, start_col) = (start_row as usize, start_col as usize);
            if index < start_row || DIVIDEND_AMOUNT < start_col {
                return None;
            }
            range
                .get((index - start_row, DIVIDEND_AMOUNT - start_col))
                .map(|formula| formula.trim().to_string())
                .filter(|formula| !formula.is_empty())
                .map(|formula| {
                    if formula.starts_with('=') {
                        formula
                    } else {
                        format!("={formula}")
                    }
                })
        });
        dividends.push(SheetDividend {
            code,
            pay_date,
            amount,
            rate_on_cost: number(row, DIVIDEND_RATE_COST),
            rate_on_price: number(row, DIVIDEND_RATE_PRICE),
            shares: number(row, DIVIDEND_SHARES),
            amount_formula,
            source_row: index + 1,
        });
    }
    dividends
}

/// Column layout of a market's trade sheet.
struct TradeColumns {
    code: usize,
    shares: usize,
    total: usize,
    unit_price: usize,
    fee: Option<usize>,
    date: usize,
    trade_type: usize,
}

fn trade_columns(market: Market) -> TradeColumns {
    match market {
        // 港股Trade: A 股票代碼 B 股數 C(unit incl fee) D buy total E 單價 F G fee H 日期 I 類別
        Market::Hk => TradeColumns {
            code: 0,
            shares: 1,
            total: 3,
            unit_price: 4,
            fee: Some(6),
            date: 7,
            trade_type: 8,
        },
        // 美股Trade: A 股票代碼 B 股數 C(unit incl fee) D buy total E 單價 F fee G 日期 H 類別
        Market::Us => TradeColumns {
            code: 0,
            shares: 1,
            total: 3,
            unit_price: 4,
            fee: Some(5),
            date: 6,
            trade_type: 7,
        },
    }
}

fn header_row(rows: &Rows, column: usize, header: &str) -> Option<usize> {
    rows.iter().position(
        |row| matches!(cell(row, column), Some(Data::String(value)) if value.trim() == header),
    )
}

fn parse_trades(rows: &Rows, market: Market) -> anyhow::Result<Vec<SheetTrade>> {
    let columns = trade_columns(market);
    let header = header_row(rows, columns.code, "股票代碼")
        .ok_or_else(|| anyhow!("trade sheet for {} has no 股票代碼 header", market.as_str()))?;

    let mut trades = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header + 1) {
        let Some(code) = text(row, columns.code) else {
            continue; // blank row, or a row that only holds the 派息 columns
        };
        let source_row = offset + 1;
        let shares = number(row, columns.shares).unwrap_or(0.0);
        let unit_price = number(row, columns.unit_price).unwrap_or(0.0);
        let total = number(row, columns.total);
        let fee = columns.fee.and_then(|index| number(row, index));
        let trade_type = text(row, columns.trade_type).unwrap_or_else(|| "BUY".to_string());
        let trade_date = date(row, columns.date).ok_or_else(|| {
            anyhow!(
                "row {source_row} of the {} trade sheet has no usable 日期",
                market.as_str()
            )
        })?;

        // The buy total is authoritative whenever the sheet carries it; that
        // also covers adjustment rows that have a total but no fee or 單價.
        let (input_mode, total, fee) = match (total, fee) {
            (Some(total), _) => (InputMode::HkTotal, Some(total), None),
            (None, Some(fee)) => (InputMode::UsFee, None, Some(fee)),
            (None, None) => {
                return Err(anyhow!(
                    "row {source_row} of the {} trade sheet has neither buy total nor fee",
                    market.as_str()
                ))
            }
        };

        let note = if shares == 0.0 {
            Some(format!(
                "imported from {} row {source_row}: adjustment row with 股數 0",
                match market {
                    Market::Hk => HK_TRADE_SHEET,
                    Market::Us => US_TRADE_SHEET,
                }
            ))
        } else {
            None
        };

        trades.push(SheetTrade {
            code,
            shares,
            unit_price,
            total,
            fee,
            trade_date,
            trade_type,
            input_mode,
            note,
            source_row,
        });
    }
    Ok(trades)
}

// 港股 sheet: A sector, B 股票名稱, C ticker, D 股數, E 加權平均買入單價, F 總買入成本
fn parse_hk_stocks(rows: &Rows) -> Vec<SheetStock> {
    let Some(header) = header_row(rows, 1, "股票名稱") else {
        return Vec::new();
    };
    let mut stocks = Vec::new();
    for (index, row) in rows.iter().skip(header + 1).enumerate() {
        let Some(code) = text(row, 1) else { break };
        stocks.push(SheetStock {
            code,
            ticker: text(row, 2),
            exchange: Some("HKG".to_string()),
            sector: text(row, 0),
            sort_order: index as i64 + 1,
        });
    }
    stocks
}

// 美股 sheet: A strategy/sector, B exchange, C 股票代碼, D 股數, E avg, F cost
fn parse_us_stocks(rows: &Rows) -> Vec<SheetStock> {
    let Some(header) = header_row(rows, 2, "股票代碼") else {
        return Vec::new();
    };
    let mut stocks = Vec::new();
    for (index, row) in rows.iter().skip(header + 1).enumerate() {
        let Some(code) = text(row, 2) else { break };
        stocks.push(SheetStock {
            ticker: Some(code.clone()),
            code,
            exchange: text(row, 1),
            sector: text(row, 0),
            sort_order: index as i64 + 1,
        });
    }
    stocks
}

fn parse_summary(rows: &Rows, market: Market) -> Vec<SheetSummary> {
    let (code_column, header) = match market {
        Market::Hk => (1usize, "股票名稱"),
        Market::Us => (2usize, "股票代碼"),
    };
    let Some(header) = header_row(rows, code_column, header) else {
        return Vec::new();
    };
    let mut summary = Vec::new();
    for row in rows.iter().skip(header + 1) {
        let Some(code) = text(row, code_column) else {
            break;
        };
        summary.push(SheetSummary {
            code,
            shares_held: number(row, 3),
            weighted_avg_buy_price: number(row, 4),
            total_buy_cost: number(row, 5),
        });
    }
    summary
}

// 定期Info 表_定期List: status | id | input | rate | 利息 | total | end date |
// note1 | note2 | month | year — located by header names, not a fixed range.
fn parse_deposits(rows: &Rows) -> anyhow::Result<Vec<SheetDeposit>> {
    let header = rows
        .iter()
        .position(|row| {
            row.iter()
                .any(|c| matches!(c, Data::String(v) if v.trim() == "status"))
                && row
                    .iter()
                    .any(|c| matches!(c, Data::String(v) if v.trim() == "input"))
        })
        .ok_or_else(|| anyhow!("{DEPOSIT_INFO_SHEET} sheet has no 表_定期List header"))?;

    let mut columns: HashMap<String, usize> = HashMap::new();
    for (index, cell) in rows[header].iter().enumerate() {
        if let Data::String(name) = cell {
            columns.insert(name.trim().to_string(), index);
        }
    }
    let column = |name: &str| -> anyhow::Result<usize> {
        columns
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("{DEPOSIT_INFO_SHEET} 表_定期List has no {name} column"))
    };
    let id_col = column("id")?;
    let input_col = column("input")?;
    let rate_col = column("rate")?;
    let interest_col = column("利息")?;
    let end_date_col = column("end date")?;
    let note1_col = columns.get("note1").copied();
    let note2_col = columns.get("note2").copied();

    let mut deposits = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header + 1) {
        let label = text(row, id_col);
        let principal = number(row, input_col);
        let interest = number(row, interest_col);
        let end_date = date(row, end_date_col);
        if label.is_none() && principal.is_none() && interest.is_none() && end_date.is_none() {
            continue; // blank row outside the table
        }
        let source_row = offset + 1;
        let Some(end_date) = end_date else {
            return Err(anyhow!(
                "row {source_row} of {DEPOSIT_INFO_SHEET} has no usable end date"
            ));
        };
        deposits.push(SheetDeposit {
            label,
            principal,
            rate: number(row, rate_col),
            interest,
            end_date,
            note1: note1_col.and_then(|col| text(row, col)),
            note2: note2_col.and_then(|col| text(row, col)),
            sort_order: deposits.len() as i64 + 1,
            source_row,
        });
    }
    Ok(deposits)
}

/// The "1月".."12月" row labels used by the 定期 month table and the
/// 定期Info year tables.
fn month_number(label: &str) -> Option<u32> {
    let month = label.strip_suffix('月')?.parse::<u32>().ok()?;
    (1..=12).contains(&month).then_some(month)
}

/// Cached aggregate cells: the 定期 sheet's B1 total, month table and bank
/// rows, plus the 定期Info year tables. Anything without all three numbers is
/// skipped, which keeps the checklists and scratch cells out.
fn parse_deposit_cached(view: &Rows, info: &Rows) -> DepositSheetCached {
    let mut cached = DepositSheetCached {
        active_principal: view.first().and_then(|row| number(row, 1)),
        ..DepositSheetCached::default()
    };

    for row in view {
        let Some(Data::String(label)) = cell(row, 0) else {
            continue;
        };
        let label = label.trim();
        let (Some(total), Some(interest), Some(principal)) =
            (number(row, 1), number(row, 2), number(row, 3))
        else {
            continue;
        };
        let sums = SheetActiveSums {
            total,
            interest,
            principal,
        };
        if let Some(month) = month_number(label) {
            cached.months.push((month, sums));
        } else if label.eq_ignore_ascii_case("total") {
            cached.grand_total = Some(sums);
        } else {
            cached.banks.push((label.to_string(), sums));
        }
    }

    // A "YYYY" label with B/C/D = Total/利息/定期 starts a 12-row month block.
    for (index, row) in info.iter().enumerate() {
        let Some(year) = text(row, 0)
            .and_then(|value| value.parse::<i32>().ok())
            .filter(|year| (2000..=2100).contains(year))
        else {
            continue;
        };
        if text(row, 1).as_deref() != Some("Total") {
            continue;
        }
        let mut months = Vec::new();
        for month in 1..=12u32 {
            let Some(month_row) = info.get(index + month as usize) else {
                break;
            };
            if text(month_row, 0).and_then(|label| month_number(&label)) != Some(month) {
                break;
            }
            months.push((
                month,
                SheetYearSums {
                    total: number(month_row, 1).unwrap_or(0.0),
                    interest: number(month_row, 2).unwrap_or(0.0),
                    payout: number(month_row, 3).unwrap_or(0.0),
                },
            ));
        }
        if !months.is_empty() {
            cached.years.push(SheetYearTable { year, months });
        }
    }
    cached
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workbook_path() -> std::path::PathBuf {
        // The crate lives one level below the workspace root.
        std::path::PathBuf::from("../財富分析報告.xlsx")
    }

    #[test]
    fn reads_both_trade_sheets_with_cached_values() {
        let data = read(&workbook_path()).expect("workbook is readable");

        assert_eq!(data.hk.trades.len(), 24);
        assert_eq!(data.us.trades.len(), 10);

        let first = &data.hk.trades[0];
        assert_eq!(first.code, "中國銀行");
        assert_eq!(first.trade_date, "2023-05-23");
        assert_eq!(first.trade_type, "BUY");
        assert_eq!(first.shares, 30000.0);
        assert_eq!(first.total, Some(96523.15));

        // 港燈 2024-04-05 holds the formula =32825.78+70.
        let hk_light = data
            .hk
            .trades
            .iter()
            .find(|t| t.code == "港燈" && t.trade_date == "2024-04-05")
            .expect("港燈 trade");
        assert_eq!(hk_light.total, Some(32895.78));

        // The US adjustment row: 0 shares, total -0.01, and an explaining note.
        let adjustment = data
            .us
            .trades
            .iter()
            .find(|t| t.shares == 0.0)
            .expect("adjustment row");
        assert_eq!(adjustment.total, Some(-0.01));
        assert!(adjustment.note.is_some());
    }

    #[test]
    fn reads_stock_metadata_including_stocks_without_trades() {
        let data = read(&workbook_path()).expect("workbook is readable");

        let gas = data
            .hk
            .stocks
            .iter()
            .find(|s| s.code == "香港中華煤氣")
            .expect("煤氣");
        assert_eq!(gas.ticker.as_deref(), Some("0003"));
        assert_eq!(gas.exchange.as_deref(), Some("HKG"));
        assert_eq!(gas.sector.as_deref(), Some("Utilities"));

        let voo = data
            .us
            .stocks
            .iter()
            .find(|s| s.code == "VOO")
            .expect("VOO");
        assert_eq!(voo.exchange.as_deref(), Some("NYSEARCA"));

        // Listed on 港股 but with no trade rows.
        assert!(data.hk.stocks.iter().any(|s| s.code == "ＦＧ恆生紅利"));
        assert!(!data.hk.trades.iter().any(|t| t.code == "ＦＧ恆生紅利"));
    }

    #[test]
    fn reads_cached_summary_values() {
        let data = read(&workbook_path()).expect("workbook is readable");
        let boc = data
            .hk
            .summary
            .iter()
            .find(|s| s.code == "中國銀行")
            .expect("中國銀行");
        assert_eq!(boc.shares_held, Some(68000.0));
        assert_eq!(boc.total_buy_cost, Some(208746.64));
        assert!((boc.weighted_avg_buy_price.unwrap() - 3.069803529).abs() < 1e-6);

        // A stock listed without cached figures reports them as absent.
        let unpriced = data
            .hk
            .summary
            .iter()
            .find(|s| s.code == "ＦＧ恆生紅利")
            .expect("ＦＧ恆生紅利");
        assert_eq!(unpriced.total_buy_cost, None);
    }

    #[test]
    fn reads_the_deposit_list_and_cached_aggregates() {
        let data = read(&workbook_path()).expect("workbook is readable");
        assert_eq!(data.deposits.len(), 23);

        let sc9632 = data
            .deposits
            .iter()
            .find(|d| d.label.as_deref() == Some("SC-9632"))
            .expect("SC-9632");
        assert_eq!(sc9632.principal, Some(110000.0));
        assert_eq!(sc9632.end_date, "2026-10-12");
        assert_eq!(sc9632.sort_order, 18);

        // Formula input =90000-32000 imports as its computed value.
        let sc4501 = data
            .deposits
            .iter()
            .find(|d| d.label.as_deref() == Some("SC-4501"))
            .expect("SC-4501");
        assert_eq!(sc4501.principal, Some(58000.0));

        // Interest-only row: no label, no principal.
        assert!(data
            .deposits
            .iter()
            .any(|d| d.label.is_none() && d.principal.is_none() && d.interest == Some(539.25)));

        // 定期!B1 and the month table (10月: Total 146278 / 利息 1278 / 定期 145000).
        assert_eq!(data.deposit_cached.active_principal, Some(445000.0));
        let (_, oct) = data
            .deposit_cached
            .months
            .iter()
            .find(|(month, _)| *month == 10)
            .expect("10月");
        assert_eq!(oct.principal, 145000.0);
        assert_eq!(oct.interest, 1278.0);
        assert_eq!(oct.total, 146278.0);

        let (_, sc) = data
            .deposit_cached
            .banks
            .iter()
            .find(|(prefix, _)| prefix == "SC")
            .expect("SC bank row");
        assert_eq!(sc.principal, 365000.0);

        // 表_2027定期 1月: Total 81614 / 利息 807 / 定期 80807.
        let y2027 = data
            .deposit_cached
            .years
            .iter()
            .find(|table| table.year == 2027)
            .expect("2027 table");
        let (_, jan) = y2027
            .months
            .iter()
            .find(|(month, _)| *month == 1)
            .expect("1月");
        assert_eq!(jan.interest, 807.0);
        assert_eq!(jan.payout, 80807.0);
        assert_eq!(jan.total, 81614.0);
    }

    #[test]
    fn missing_workbook_names_the_file() {
        let err = read(std::path::Path::new("/tmp/does-not-exist.xlsx")).expect_err("must fail");
        assert!(err.to_string().contains("does-not-exist.xlsx"));
    }
}
