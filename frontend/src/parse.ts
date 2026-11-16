/** Input-side parsing helpers (display formatting lives in format.ts). */

/** Blank/non-numeric input is `null` so the API sees an absent field. */
export function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}
