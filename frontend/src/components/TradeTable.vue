<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { api, ApiError, type Stock, type Trade, type TradeType } from '../api'
import { fmtMoney, fmtPrice, fmtShares } from '../format'

const props = defineProps<{
  trades: Trade[]
  stocks: Stock[]
  order: 'asc' | 'desc'
}>()

const emit = defineEmits<{
  saved: [Trade]
  remove: [Trade]
  'toggle-order': []
  error: [string]
}>()

interface Draft {
  stock_id: string
  trade_type: TradeType
  trade_date: string
  shares: string
  unit_price: string
  total: string
  fee: string
  note: string
}

const editingId = ref<number | null>(null)
const saving = ref(false)
const rowError = ref('')
const draft = reactive<Draft>({
  stock_id: '',
  trade_type: 'BUY',
  trade_date: '',
  shares: '',
  unit_price: '',
  total: '',
  fee: '',
  note: '',
})

const totalCost = computed(() =>
  props.trades
    .filter((trade) => trade.trade_type === 'BUY')
    .reduce((sum, trade) => sum + trade.total, 0),
)

function num(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

function startEdit(trade: Trade) {
  editingId.value = trade.id
  rowError.value = ''
  Object.assign(draft, {
    stock_id: String(trade.stock_id),
    trade_type: trade.trade_type,
    trade_date: trade.trade_date,
    shares: String(trade.shares),
    unit_price: String(trade.unit_price),
    total: String(trade.total),
    fee: String(trade.fee),
    note: trade.note ?? '',
  })
}

function cancelEdit() {
  editingId.value = null
  rowError.value = ''
}

const preview = computed(() => {
  const trade = props.trades.find((item) => item.id === editingId.value)
  const shares = num(draft.shares)
  const unitPrice = num(draft.unit_price)
  if (!trade || shares === null || unitPrice === null) return null

  if (trade.input_mode === 'HK_TOTAL') {
    const total = num(draft.total)
    if (total === null) return null
    return {
      fee: total - shares * unitPrice,
      total,
      unitInclFee: shares === 0 ? null : total / shares,
    }
  }

  const fee = num(draft.fee)
  if (fee === null) return null
  const total = shares * unitPrice + fee
  return { fee, total, unitInclFee: shares === 0 ? null : total / shares }
})

async function saveEdit(trade: Trade) {
  rowError.value = ''
  saving.value = true
  try {
    const isHk = trade.input_mode === 'HK_TOTAL'
    const saved = await api.updateTrade(trade.id, {
      stock_id: Number(draft.stock_id),
      trade_type: draft.trade_type,
      trade_date: draft.trade_date,
      shares: num(draft.shares) ?? 0,
      unit_price: num(draft.unit_price) ?? 0,
      total: isHk ? num(draft.total) : null,
      fee: isHk ? null : num(draft.fee),
      input_mode: trade.input_mode,
      note: String(draft.note).trim() === '' ? null : String(draft.note).trim(),
    })
    editingId.value = null
    emit('saved', saved)
  } catch (err) {
    const message = err instanceof ApiError ? err.message : String(err)
    rowError.value = message
    emit('error', message)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <table>
    <thead>
      <tr>
        <th class="sortable" @click="emit('toggle-order')">
          日期 <span aria-hidden="true">{{ props.order === 'asc' ? '▲' : '▼' }}</span>
        </th>
        <th>股票代碼</th>
        <th>類別</th>
        <th class="num">股數</th>
        <th class="num">單價</th>
        <th class="num">fee</th>
        <th class="num">buy total</th>
        <th class="num" title="buy total ÷ 股數">平均單價（含 fee）</th>
        <th>備註</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <template v-for="trade in props.trades" :key="trade.id">
        <tr v-if="editingId === trade.id" class="editing">
          <td><input v-model="draft.trade_date" type="date" /></td>
          <td>
            <select v-model="draft.stock_id">
              <option v-for="stock in props.stocks" :key="stock.id" :value="String(stock.id)">
                {{ stock.code }}
              </option>
            </select>
          </td>
          <td>
            <select v-model="draft.trade_type">
              <option value="BUY">BUY</option>
              <option value="SELL">SELL</option>
            </select>
          </td>
          <td class="num"><input v-model="draft.shares" type="number" step="any" /></td>
          <td class="num"><input v-model="draft.unit_price" type="number" step="any" /></td>
          <td class="num">
            <input v-if="trade.input_mode === 'US_FEE'" v-model="draft.fee" type="number" step="any" />
            <span v-else>{{ fmtMoney(preview?.fee ?? trade.fee) }}</span>
          </td>
          <td class="num">
            <input v-if="trade.input_mode === 'HK_TOTAL'" v-model="draft.total" type="number" step="any" />
            <span v-else>{{ fmtMoney(preview?.total ?? trade.total) }}</span>
          </td>
          <td class="num">{{ fmtPrice(preview?.unitInclFee) }}</td>
          <td><input v-model="draft.note" type="text" /></td>
          <td class="row-actions">
            <button type="button" class="link" :disabled="saving" @click="saveEdit(trade)">儲存</button>
            <button type="button" class="link" :disabled="saving" @click="cancelEdit">取消</button>
          </td>
        </tr>
        <tr v-if="editingId === trade.id && rowError">
          <td colspan="10" class="error">{{ rowError }}</td>
        </tr>
        <tr v-else-if="editingId !== trade.id">
          <td>{{ trade.trade_date }}</td>
          <td>{{ trade.code }}</td>
          <td>{{ trade.trade_type }}</td>
          <td class="num">{{ fmtShares(trade.shares) }}</td>
          <td class="num">{{ fmtPrice(trade.unit_price) }}</td>
          <td class="num">{{ fmtMoney(trade.fee) }}</td>
          <td class="num">{{ fmtMoney(trade.total) }}</td>
          <td class="num">{{ fmtPrice(trade.unit_price_incl_fee) }}</td>
          <td class="note">{{ trade.note ?? '' }}</td>
          <td class="row-actions">
            <button type="button" class="link" @click="startEdit(trade)">編輯</button>
            <button type="button" class="link danger" @click="emit('remove', trade)">刪除</button>
          </td>
        </tr>
      </template>
      <tr v-if="props.trades.length === 0">
        <td colspan="10" class="muted">沒有符合條件的交易</td>
      </tr>
    </tbody>
    <tfoot v-if="props.trades.length > 0">
      <tr>
        <td colspan="6">{{ props.trades.length }} 筆交易</td>
        <td class="num">{{ fmtMoney(totalCost) }}</td>
        <td colspan="3" class="muted">BUY 合計</td>
      </tr>
    </tfoot>
  </table>
</template>

<style scoped>
.sortable {
  cursor: pointer;
  user-select: none;
}
.editing input,
.editing select {
  width: 7rem;
  padding: 0.2rem 0.3rem;
}
.editing input[type='date'] {
  width: 9rem;
}
.note {
  max-width: 16rem;
  font-size: 0.8rem;
}
.row-actions {
  white-space: nowrap;
}
</style>
