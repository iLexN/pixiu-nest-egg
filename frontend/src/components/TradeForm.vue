<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { api, ApiError, type Market, type Stock, type Trade, type TradeType } from '../api'
import DateInput from './DateInput.vue'
import { fmtMoney, fmtPrice, todayIso } from '../format'

const props = defineProps<{
  market: Market
  stocks: Stock[]
  editing: Trade | null
}>()

const emit = defineEmits<{ saved: [Trade]; cancelled: [] }>()

interface FormState {
  code: string
  trade_type: TradeType
  trade_date: string
  shares: string
  unit_price: string
  total: string
  fee: string
  note: string
}

function emptyForm(): FormState {
  return {
    code: props.stocks[0]?.code ?? '',
    trade_type: 'BUY',
    trade_date: todayIso(),
    shares: '',
    unit_price: '',
    total: '',
    fee: '',
    note: '',
  }
}

const form = reactive<FormState>(emptyForm())
const error = ref<ApiError | null>(null)
const saving = ref(false)

watch(
  () => props.stocks,
  (stocks) => {
    if (!stocks.some((stock) => stock.code === form.code)) {
      form.code = stocks[0]?.code ?? ''
    }
  },
)

watch(
  () => props.editing,
  (trade) => {
    if (!trade) {
      Object.assign(form, emptyForm())
      return
    }
    Object.assign(form, {
      code: trade.code,
      trade_type: trade.trade_type,
      trade_date: trade.trade_date,
      shares: String(trade.shares),
      unit_price: String(trade.unit_price),
      total: String(trade.total),
      fee: String(trade.fee),
      note: trade.note ?? '',
    })
  },
  { immediate: true },
)

watch(
  () => props.market,
  () => Object.assign(form, emptyForm()),
)

const isHk = computed(() => props.market === 'HK')

function num(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

const preview = computed(() => {
  const shares = num(form.shares)
  const unitPrice = num(form.unit_price)
  if (shares === null || unitPrice === null) return null

  if (isHk.value) {
    const total = num(form.total)
    if (total === null) return null
    return {
      fee: total - shares * unitPrice,
      total,
      unitInclFee: shares === 0 ? null : total / shares,
    }
  }
  const fee = num(form.fee)
  if (fee === null) return null
  const total = shares * unitPrice + fee
  return { fee, total, unitInclFee: shares === 0 ? null : total / shares }
})

async function submit() {
  error.value = null
  saving.value = true
  try {
    const shares = num(form.shares) ?? 0
    const unitPrice = num(form.unit_price) ?? 0
    const note = String(form.note).trim() === '' ? null : String(form.note).trim()
    let saved: Trade
    if (props.editing) {
      const stockId =
        props.stocks.find((stock) => stock.code === form.code)?.id ?? props.editing.stock_id
      saved = await api.updateTrade(props.editing.id, {
        stock_id: stockId,
        trade_type: form.trade_type,
        trade_date: form.trade_date,
        shares,
        unit_price: unitPrice,
        total: isHk.value ? num(form.total) : null,
        fee: isHk.value ? null : num(form.fee),
        input_mode: props.editing.input_mode,
        note,
      })
    } else {
      saved = await api.createTrade({
        market: props.market,
        code: form.code,
        trade_type: form.trade_type,
        trade_date: form.trade_date,
        shares,
        unit_price: unitPrice,
        total: isHk.value ? num(form.total) : null,
        fee: isHk.value ? null : num(form.fee),
        input_mode: isHk.value ? 'HK_TOTAL' : 'US_FEE',
        note,
      })
      Object.assign(form, emptyForm())
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
  <form class="card trade-form" @submit.prevent="submit">
    <h3>{{ props.editing ? `編輯交易 #${props.editing.id}` : '新增交易' }}</h3>

    <div class="grid">
      <label>
        股票代碼
        <select v-model="form.code" required>
          <option v-for="stock in props.stocks" :key="stock.id" :value="stock.code">
            {{ stock.code }}
          </option>
        </select>
        <small v-if="error?.fieldMessage('code')" class="error">{{
          error.fieldMessage('code')
        }}</small>
      </label>

      <label>
        類別
        <select v-model="form.trade_type">
          <option value="BUY">BUY</option>
          <option value="SELL">SELL</option>
        </select>
      </label>

      <label>
        日期
        <DateInput v-model="form.trade_date" required />
        <small v-if="error?.fieldMessage('trade_date')" class="error">{{
          error.fieldMessage('trade_date')
        }}</small>
      </label>

      <label>
        股數
        <input v-model="form.shares" type="number" step="any" inputmode="decimal" required />
        <small v-if="error?.fieldMessage('shares')" class="error">{{
          error.fieldMessage('shares')
        }}</small>
      </label>

      <label>
        單價
        <input v-model="form.unit_price" type="number" step="any" inputmode="decimal" required />
        <small v-if="error?.fieldMessage('unit_price')" class="error">{{
          error.fieldMessage('unit_price')
        }}</small>
      </label>

      <label v-if="isHk">
        buy total（已含 fee）
        <input v-model="form.total" type="number" step="any" inputmode="decimal" required />
        <small v-if="error?.fieldMessage('total')" class="error">{{
          error.fieldMessage('total')
        }}</small>
      </label>

      <label v-else>
        fee
        <input v-model="form.fee" type="number" step="any" inputmode="decimal" required />
        <small v-if="error?.fieldMessage('fee')" class="error">{{
          error.fieldMessage('fee')
        }}</small>
      </label>

      <label class="wide">
        備註
        <input v-model="form.note" type="text" placeholder="調整或負 fee 時必填" />
      </label>
    </div>

    <p v-if="preview" class="preview">
      <span v-if="isHk">fee <strong>{{ fmtMoney(preview.fee) }}</strong></span>
      <span v-else>buy total <strong>{{ fmtMoney(preview.total) }}</strong></span>
      <span>平均單價（含 fee） <strong>{{ fmtPrice(preview.unitInclFee) }}</strong></span>
    </p>
    <p v-else class="preview muted">填入 股數、單價 及 {{ isHk ? 'buy total' : 'fee' }} 後顯示計算結果</p>

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
.trade-form h3 {
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
