<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { api, ApiError, type Dividend, type SummaryStock } from '../api'
import DateInput from './DateInput.vue'
import { fmtMoney, fmtPrice, fmtShares, todayIso } from '../format'

const props = defineProps<{
  stocks: SummaryStock[]
  editing: Dividend | null
}>()

const emit = defineEmits<{ saved: [Dividend]; cancelled: [] }>()

interface FormState {
  stock_id: number | null
  pay_date: string
  per_share: string
  estimated_amount: string
  received_amount: string
  received_price: string
  note: string
  refresh_snapshots: boolean
}

function emptyForm(): FormState {
  return {
    stock_id: props.stocks[0]?.id ?? null,
    pay_date: todayIso(),
    per_share: '',
    estimated_amount: '',
    received_amount: '',
    received_price: '',
    note: '',
    refresh_snapshots: false,
  }
}

const form = reactive<FormState>(emptyForm())
const error = ref<ApiError | null>(null)
const saving = ref(false)

watch(
  () => props.stocks,
  (stocks) => {
    if (!stocks.some((stock) => stock.id === form.stock_id)) {
      form.stock_id = stocks[0]?.id ?? null
    }
  },
)

// Inputs can't show the stored f64 verbatim (e.g. 0.22557833333333335);
// trim to 6 decimals for display.
function toInput(value: number): string {
  return String(Number(value.toFixed(6)))
}

watch(
  () => props.editing,
  (dividend) => {
    if (!dividend) {
      Object.assign(form, emptyForm())
      return
    }
    Object.assign(form, {
      stock_id: dividend.stock_id,
      pay_date: dividend.pay_date,
      per_share: dividend.per_share === null ? '' : toInput(dividend.per_share),
      estimated_amount:
        dividend.estimated_amount === null ? '' : toInput(dividend.estimated_amount),
      received_amount:
        dividend.received_amount === null ? '' : toInput(dividend.received_amount),
      received_price:
        dividend.received_price === null ? '' : toInput(dividend.received_price),
      note: dividend.note ?? '',
      refresh_snapshots: false,
    })
  },
  { immediate: true },
)

// v-model on type="number" casts to number, so accept both.
function num(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

const selectedStock = computed(
  () => props.stocks.find((stock) => stock.id === form.stock_id) ?? null,
)

// When editing, the record's frozen snapshot is the denominator; for a new
// record the stock's current holdings approximate the snapshot the server
// will freeze at pay_date.
const snapshotShares = computed(
  () => props.editing?.shares_held ?? selectedStock.value?.shares_held,
)

// Non-authoritative hints only; the server freezes its own snapshot on save.
const preview = computed(() => {
  const perShare = num(form.per_share)
  const shares = snapshotShares.value
  if (perShare === null || shares === undefined) return null
  return perShare * shares
})

const perSharePreview = computed(() => {
  const amount = num(form.estimated_amount)
  const shares = snapshotShares.value
  if (amount === null || !shares) return null
  return amount / shares
})

async function submit() {
  error.value = null
  saving.value = true
  try {
    const payload = {
      pay_date: form.pay_date,
      per_share: num(form.per_share),
      estimated_amount: num(form.estimated_amount),
      note: form.note.trim() === '' ? null : form.note.trim(),
    }
    const saved = props.editing
      ? await api.updateDividend(props.editing.id, {
          ...payload,
          // Receipt fields only appear when the record is already received.
          ...(props.editing.status === 'RECEIVED'
            ? {
                received_amount: num(form.received_amount),
                received_price: num(form.received_price),
              }
            : {}),
          refresh_snapshots: form.refresh_snapshots || undefined,
        })
      : await api.createDividend({
          stock_id: form.stock_id ?? undefined,
          ...payload,
        })
    if (!props.editing) {
      const stockId = form.stock_id
      Object.assign(form, emptyForm())
      form.stock_id = stockId
    }
    emit('saved', saved)
  } catch (err) {
    error.value =
      err instanceof ApiError ? err : new ApiError(0, (err as Error).message ?? 'request failed', [])
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="card dividend-form" @submit.prevent="submit">
    <h3>
      {{
        props.editing
          ? `編輯派息 ${props.editing.code}（${props.editing.pay_date}）`
          : '記錄派息'
      }}
    </h3>

    <div class="grid">
      <label>
        股票
        <select v-model="form.stock_id" :disabled="!!props.editing" required>
          <option v-for="stock in props.stocks" :key="stock.id" :value="stock.id">
            {{ stock.code }}
          </option>
        </select>
        <small v-if="error?.fieldMessage('code')" class="error">{{
          error.fieldMessage('code')
        }}</small>
      </label>

      <label>
        派息日
        <DateInput v-model="form.pay_date" required />
        <small v-if="error?.fieldMessage('pay_date')" class="error">{{
          error.fieldMessage('pay_date')
        }}</small>
      </label>

      <label>
        每股派息
        <input
          v-model="form.per_share"
          type="number"
          step="any"
          inputmode="decimal"
          :placeholder="perSharePreview === null ? '' : fmtPrice(perSharePreview)"
        />
        <small v-if="error?.fieldMessage('per_share')" class="error">{{
          error.fieldMessage('per_share')
        }}</small>
      </label>

      <label>
        預期派息
        <input
          v-model="form.estimated_amount"
          type="number"
          step="any"
          inputmode="decimal"
          :placeholder="preview === null ? '' : fmtMoney(preview)"
        />
        <small v-if="error?.fieldMessage('estimated_amount')" class="error">{{
          error.fieldMessage('estimated_amount')
        }}</small>
      </label>

      <template v-if="props.editing?.status === 'RECEIVED'">
        <label>
          實收派息
          <input
            v-model="form.received_amount"
            type="number"
            step="any"
            inputmode="decimal"
          />
          <small v-if="error?.fieldMessage('received_amount')" class="error">{{
            error.fieldMessage('received_amount')
          }}</small>
        </label>

        <label>
          當時現價
          <input
            v-model="form.received_price"
            type="number"
            step="any"
            inputmode="decimal"
          />
          <small v-if="error?.fieldMessage('received_price')" class="error">{{
            error.fieldMessage('received_price')
          }}</small>
        </label>
      </template>

      <label class="wide">
        note
        <input v-model="form.note" type="text" placeholder="匯率、手續費等假設" />
      </label>
    </div>

    <p v-if="preview !== null && form.estimated_amount === ''" class="preview">
      <span
        >預估 {{ fmtShares(snapshotShares) }} 股 →
        <strong>{{ fmtMoney(preview) }}</strong></span
      >
    </p>
    <p v-else-if="perSharePreview !== null && form.per_share === ''" class="preview">
      <span
        >{{ fmtMoney(num(form.estimated_amount)) }} ÷
        {{ fmtShares(snapshotShares) }} 股 → 每股
        <strong>{{ fmtPrice(perSharePreview) }}</strong></span
      >
    </p>
    <p v-else class="preview muted">填入 每股派息 或 預期派息</p>

    <label v-if="props.editing" class="checkbox">
      <input v-model="form.refresh_snapshots" type="checkbox" />
      依派息日重新計算 股數／總買入成本 快照
    </label>

    <p v-if="error && error.fields.length === 0" class="error">{{ error.message }}</p>

    <div class="actions">
      <button type="submit" :disabled="saving">{{ props.editing ? '儲存' : '新增' }}</button>
      <button v-if="props.editing" type="button" class="secondary" @click="emit('cancelled')">
        取消
      </button>
    </div>
  </form>
</template>

<style scoped>
.dividend-form h3 {
  margin: 0 0 0.75rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
}
label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.25rem;
}
.wide {
  grid-column: 1 / -1;
}
.checkbox {
  flex-direction: row;
  align-items: center;
  margin-top: 0.75rem;
}
.preview {
  display: flex;
  flex-wrap: wrap;
  gap: 1.5rem;
  margin: 0.75rem 0 0;
  font-size: 0.9rem;
}
.actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
</style>
