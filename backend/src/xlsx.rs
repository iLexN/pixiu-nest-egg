//! Read-only reader for the spreadsheet being replaced.
//!
//! Only the trade tables and the per-stock metadata/summary columns are read;
//! everything else on those sheets (the 派息 table, templates, scratch cells)
//! is ignored. Formula cells are read as their cached values.

use std::path::Path;

use anyhow::{anyhow, Context};
use calamine::{open_workbook_auto, Data, Reader};

use crate::models::{InputMode, Market};

pub const HK_TRADE_SHEET: &str = "港股Trade";
pub const HK_SUMMARY_SHEET: &str = "港股";
pub const US_TRADE_SHEET: &str = "美股Trade";
pub const US_SUMMARY_SHEET: &str = "美股";

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

#[derive(Debug, Clone)]
pub struct MarketSheets {
    pub market: Market,
    pub trades: Vec<SheetTrade>,
    pub stocks: Vec<SheetStock>,
    pub summary: Vec<SheetSummary>,
}

#[derive(Debug, Clone)]
pub struct WorkbookData {
    pub hk: MarketSheets,
    pub us: MarketSheets,
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

    Ok(WorkbookData {
        hk: MarketSheets {
            market: Market::Hk,
            trades: parse_trades(&hk_trades, Market::Hk)?,
            stocks: parse_hk_stocks(&hk_summary),
            summary: parse_summary(&hk_summary, Market::Hk),
        },
        us: MarketSheets {
            market: Market::Us,
            trades: parse_trades(&us_trades, Market::Us)?,
            stocks: parse_us_stocks(&us_summary),
            summary: parse_summary(&us_summary, Market::Us),
        },
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
    fn missing_workbook_names_the_file() {
        let err = read(std::path::Path::new("/tmp/does-not-exist.xlsx")).expect_err("must fail");
        assert!(err.to_string().contains("does-not-exist.xlsx"));
    }
}
