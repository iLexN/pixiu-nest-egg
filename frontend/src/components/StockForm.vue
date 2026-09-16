<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { api, ApiError, type Market, type Stock } from '../api'

const props = defineProps<{
  market: Market
  editing: Stock | null
}>()

const emit = defineEmits<{
  saved: [Stock]
  cancelled: []
}>()

interface FormState {
  code: string
  ticker: string
  exchange: string
  sector: string
  manual_price: string
  pe: string
  eps: string
  high52: string
  low52: string
  note: string
}

function emptyForm(): FormState {
  return {
    code: '',
    ticker: '',
    exchange: '',
    sector: '',
    manual_price: '',
    pe: '',
    eps: '',
    high52: '',
    low52: '',
    note: '',
  }
}

const form = reactive<FormState>(emptyForm())
const error = ref<ApiError | null>(null)
const saving = ref(false)

watch(
  () => props.editing,
  (stock) => {
    if (!stock) {
      Object.assign(form, emptyForm())
      return
    }
    Object.assign(form, {
      code: stock.code,
      ticker: stock.ticker ?? '',
      exchange: stock.exchange ?? '',
      sector: stock.sector ?? '',
      manual_price: stock.manual_price === null ? '' : String(stock.manual_price),
      pe: stock.pe === null ? '' : String(stock.pe),
      eps: stock.eps === null ? '' : String(stock.eps),
      high52: stock.high52 === null ? '' : String(stock.high52),
      low52: stock.low52 === null ? '' : String(stock.low52),
      note: stock.note ?? '',
    })
  },
  { immediate: true },
)

watch(
  () => props.market,
  () => Object.assign(form, emptyForm()),
)

function optional(value: string | number | null): string | null {
  const text = String(value ?? '').trim()
  return text === '' ? null : text
}

function optionalNumber(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

async function submit() {
  error.value = null
  saving.value = true
  try {
    const payload = {
      code: String(form.code).trim(),
      ticker: optional(form.ticker),
      exchange: optional(form.exchange),
      sector: optional(form.sector),
      manual_price: optionalNumber(form.manual_price),
      pe: optionalNumber(form.pe),
      eps: optionalNumber(form.eps),
      high52: optionalNumber(form.high52),
      low52: optionalNumber(form.low52),
      note: optional(form.note),
    }
    const saved = props.editing
      ? await api.updateStock(props.editing.id, payload)
      : await api.createStock({ market: props.market, ...payload })
    if (!props.editing) Object.assign(form, emptyForm())
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
  <form class="card stock-form" @submit.prevent="submit">
    <h3>
      {{
        props.editing
          ? `編輯 ${props.editing.code}`
          : `新增${props.market === 'HK' ? '港股' : '美股'}股票`
      }}
    </h3>
    <div class="grid">
      <label>
        股票代碼
        <input v-model="form.code" type="text" required />
        <small v-if="error?.fieldMessage('code')" class="error">{{
          error.fieldMessage('code')
        }}</small>
      </label>
      <label>
        Stock / ticker
        <input v-model="form.ticker" type="text" placeholder="0003 / VOO" />
      </label>
      <label>
        交易所
        <input v-model="form.exchange" type="text" placeholder="HKG / NYSE" />
      </label>
      <label>
        類別
        <input v-model="form.sector" type="text" placeholder="Utilities" />
      </label>
      <label>
        現價
        <input v-model="form.manual_price" type="number" step="any" />
      </label>
      <label>
        PE
        <input v-model="form.pe" type="number" step="any" />
      </label>
      <label>
        EPS
        <input v-model="form.eps" type="number" step="any" />
      </label>
      <label>
        52週高
        <input v-model="form.high52" type="number" step="any" />
      </label>
      <label>
        52週低
        <input v-model="form.low52" type="number" step="any" />
      </label>
      <label class="wide">
        備註
        <input v-model="form.note" type="text" />
      </label>
    </div>
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
.stock-form h3 {
  margin: 0 0 0.75rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
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
.actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
</style>
