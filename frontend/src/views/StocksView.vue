<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue'
import { api, ApiError, type Market, type Stock } from '../api'
import { fmtPrice } from '../format'

const props = defineProps<{ market: Market }>()

const stocks = ref<Stock[]>([])
const error = ref('')
const message = ref('')
const editing = ref<Stock | null>(null)

const form = reactive({
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
})

function resetForm() {
  Object.assign(form, {
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
  })
  editing.value = null
}

async function load() {
  error.value = ''
  try {
    stocks.value = await api.listStocks(props.market)
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

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

async function save() {
  error.value = ''
  message.value = ''
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
    if (editing.value) {
      await api.updateStock(editing.value.id, payload)
      message.value = `已更新 ${payload.code}`
    } else {
      await api.createStock({ market: props.market, ...payload })
      message.value = `已新增 ${payload.code}`
    }
    resetForm()
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function startEdit(stock: Stock) {
  editing.value = stock
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
}

async function remove(stock: Stock) {
  error.value = ''
  message.value = ''
  if (!window.confirm(`刪除 ${stock.code}？`)) return
  try {
    await api.deleteStock(stock.id)
    message.value = `已刪除 ${stock.code}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
watch(() => props.market, () => {
  resetForm()
  void load()
})
</script>

<template>
  <section>
    <form class="card" @submit.prevent="save">
      <h3>{{ editing ? `編輯 ${editing.code}` : `新增${props.market === 'HK' ? '港股' : '美股'}股票` }}</h3>
      <div class="grid">
        <label>
          股票代碼
          <input v-model="form.code" type="text" required />
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
      <p v-if="message" class="ok">{{ message }}</p>
      <p v-if="error" class="error">{{ error }}</p>
      <div class="actions">
        <button type="submit">{{ editing ? '儲存' : '新增' }}</button>
        <button v-if="editing" type="button" class="secondary" @click="resetForm">取消</button>
      </div>
    </form>

    <div class="card">
      <table>
        <thead>
          <tr>
            <th>股票代碼</th>
            <th>Stock</th>
            <th>交易所</th>
            <th>類別</th>
            <th class="num">現價</th>
            <th class="num">PE</th>
            <th class="num">EPS</th>
            <th class="num">52週高</th>
            <th class="num">52週低</th>
            <th>備註</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="stock in stocks" :key="stock.id">
            <td>{{ stock.code }}</td>
            <td>{{ stock.ticker ?? '' }}</td>
            <td>{{ stock.exchange ?? '' }}</td>
            <td>{{ stock.sector ?? '' }}</td>
            <td class="num">{{ fmtPrice(stock.manual_price) }}</td>
            <td class="num">{{ fmtPrice(stock.pe) }}</td>
            <td class="num">{{ fmtPrice(stock.eps) }}</td>
            <td class="num">{{ fmtPrice(stock.high52) }}</td>
            <td class="num">{{ fmtPrice(stock.low52) }}</td>
            <td class="note">{{ stock.note ?? '' }}</td>
            <td class="row-actions">
              <button type="button" class="link" @click="startEdit(stock)">編輯</button>
              <button type="button" class="link danger" @click="remove(stock)">刪除</button>
            </td>
          </tr>
          <tr v-if="stocks.length === 0">
            <td colspan="11" class="muted">尚未有股票</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
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
.note {
  max-width: 16rem;
  font-size: 0.8rem;
}
.row-actions {
  white-space: nowrap;
}
</style>
