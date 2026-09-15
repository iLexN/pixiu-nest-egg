<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import TradeForm from '../components/TradeForm.vue'
import TradeTable from '../components/TradeTable.vue'
import { api, ApiError, type Market, type Stock, type Trade } from '../api'

const props = defineProps<{ market: Market }>()

const stocks = ref<Stock[]>([])
const trades = ref<Trade[]>([])
const message = ref('')
const error = ref('')

const filterCode = ref('')
const from = ref('')
const to = ref('')
const order = ref<'asc' | 'desc'>('asc')

async function load() {
  error.value = ''
  try {
    stocks.value = await api.listStocks(props.market)
    trades.value = await api.listTrades({
      market: props.market,
      code: filterCode.value || undefined,
      from: from.value || undefined,
      to: to.value || undefined,
      order: order.value,
    })
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function toggleOrder() {
  order.value = order.value === 'asc' ? 'desc' : 'asc'
  void load()
}

async function remove(trade: Trade) {
  if (!window.confirm(`刪除 ${trade.trade_date} ${trade.code} 的交易？`)) return
  try {
    await api.deleteTrade(trade.id)
    message.value = `已刪除交易 #${trade.id}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function onSaved(trade: Trade) {
  message.value = `已儲存交易 #${trade.id}`
  error.value = ''
  void load()
}

function onError(messageText: string) {
  message.value = ''
  error.value = messageText
}

function clearFilters() {
  filterCode.value = ''
  from.value = ''
  to.value = ''
  void load()
}

onMounted(load)
watch(() => props.market, () => {
  filterCode.value = ''
  void load()
})
</script>

<template>
  <section>
    <TradeForm :market="props.market" :stocks="stocks" @saved="onSaved" />

    <div class="card">
      <div class="filters">
        <label>
          股票代碼
          <select v-model="filterCode" @change="load">
            <option value="">全部</option>
            <option v-for="stock in stocks" :key="stock.id" :value="stock.code">
              {{ stock.code }}
            </option>
          </select>
        </label>
        <label>
          由
          <input v-model="from" type="date" @change="load" />
        </label>
        <label>
          至
          <input v-model="to" type="date" @change="load" />
        </label>
        <button type="button" class="secondary" @click="clearFilters">清除篩選</button>
      </div>

      <p v-if="message" class="ok">{{ message }}</p>
      <p v-if="error" class="error">{{ error }}</p>

      <TradeTable
        :trades="trades"
        :stocks="stocks"
        :order="order"
        @saved="onSaved"
        @remove="remove"
        @toggle-order="toggleOrder"
        @error="onError"
      />
    </div>
  </section>
</template>

<style scoped>
.filters {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.25rem;
}
</style>
