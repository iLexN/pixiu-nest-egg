/** Display rounding only: stored values keep their full precision. */

const money = new Intl.NumberFormat('en-US', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
})

const price = new Intl.NumberFormat('en-US', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 4,
})

const shares = new Intl.NumberFormat('en-US', {
  maximumFractionDigits: 4,
})

const percent = new Intl.NumberFormat('en-US', {
  style: 'percent',
  minimumFractionDigits: 3,
  maximumFractionDigits: 3,
})

/** Empty cells stay empty rather than showing a misleading 0. */
export function fmtMoney(value: number | null | undefined): string {
  return value === null || value === undefined ? '' : money.format(value)
}

export function fmtPrice(value: number | null | undefined): string {
  return value === null || value === undefined ? '' : price.format(value)
}

export function fmtShares(value: number | null | undefined): string {
  return value === null || value === undefined ? '' : shares.format(value)
}

export function fmtPercent(value: number | null | undefined): string {
  return value === null || value === undefined ? '' : percent.format(value)
}

export function fmtDateTime(value: string | null | undefined): string {
  if (!value) return ''
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString()
}

export function signClass(value: number | null | undefined): string {
  if (value === null || value === undefined || value === 0) return ''
  return value > 0 ? 'positive' : 'negative'
}

/** Green when `value` beats `reference` (now vs. last month), red when worse. */
export function compareClass(
  value: number | null | undefined,
  reference: number | null | undefined,
): string {
  if (value === null || value === undefined || reference === null || reference === undefined) {
    return ''
  }
  return signClass(value - reference)
}

/** A `percent / amount` pair like the 上月/最高 cells; `—` for missing parts. */
export function fmtFigures(
  percent: number | null | undefined,
  amount: number | null | undefined,
): string {
  return `${fmtPercent(percent) || '—'} / ${fmtMoney(amount) || '—'}`
}

/** Bank codes used by the deposit list; unknown codes display as-is. */
const BANK_NAMES: Record<string, string> = {
  SC: '渣打',
  HS: '恒生',
}

export function fmtBank(code: string | null | undefined): string {
  if (!code) return ''
  const name = BANK_NAMES[code]
  return name ? `${name} (${code})` : code
}

export function todayIso(): string {
  const now = new Date()
  const month = `${now.getMonth() + 1}`.padStart(2, '0')
  const day = `${now.getDate()}`.padStart(2, '0')
  return `${now.getFullYear()}-${month}-${day}`
}
