import { ref, type Ref } from 'vue'
import { toApiError, type ApiError } from './api'

/** `run` options — the mutation protocol. */
export interface RunOptions {
  /** Confirm text; a declined confirm is a no-op, not an error. */
  confirm?: string
  /** Success message set after the action completes. */
  message?: string
  /** Reload the loader after success (default true). */
  reload?: boolean
}

/** The mutation runner a page hands down to its panels. */
export type PageRun = <R>(
  action: () => Promise<R>,
  opts?: RunOptions,
) => Promise<R | undefined>

/**
 * The Page lifecycle (GLOSSARY.md) — one instance per page, one per form.
 * Owns the load–mutate–report protocol: data, success message, structured
 * error, reload-after-write, confirm-before-delete. Views keep only domain
 * wiring. `reload` swaps `data` on success and keeps it on failure; every
 * failure lands in `error` as an `ApiError` — views render `error?.message`,
 * forms render its field errors. A form instance passes no loader: it only
 * submits.
 */
export function usePage<T = undefined>(loader?: () => Promise<T>) {
  const data: Ref<T | null> = ref(null)
  const message = ref('')
  const error = ref<ApiError | null>(null)
  const busy = ref(false)

  async function reload(): Promise<void> {
    error.value = null
    if (!loader) return
    try {
      data.value = await loader()
    } catch (err) {
      error.value = toApiError(err)
    }
  }

  async function run<R>(action: () => Promise<R>, opts: RunOptions = {}): Promise<R | undefined> {
    if (opts.confirm !== undefined && !window.confirm(opts.confirm)) return undefined
    busy.value = true
    error.value = null
    message.value = ''
    try {
      const result = await action()
      if (opts.message !== undefined) message.value = opts.message
      if (opts.reload !== false) await reload()
      return result
    } catch (err) {
      error.value = toApiError(err)
      return undefined
    } finally {
      busy.value = false
    }
  }

  return { data, message, error, busy, reload, run }
}
