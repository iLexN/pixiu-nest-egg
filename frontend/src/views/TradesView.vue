<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import DateInput from '../components/DateInput.vue'
import TradeForm from '../components/TradeForm.vue'
import TradeTable from '../components/TradeTable.vue'
import { api, type Market, type Stock, type Trade } from '../api'
import { usePage } from '../page'

const props = defineProps<{ market: Market }>()

const stocks = ref<Stock[]>([])
const trades = ref<Trade[]>([])
const editing = ref<Trade | null>(null)

const showForm = ref(false)

const filterCode = ref('')
const from = ref('')
const to = ref('')
const order = ref<'asc' | 'desc'>('desc')

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/

const { message, error, reload, run } = usePage(async () => {
  stocks.value = await api.listStocks(props.market)
  trades.value = await api.listTrades({
    market: props.market,
    code: filterCode.value || undefined,
    from: ISO_DATE.test(from.value) ? from.value : undefined,
    to: ISO_DATE.test(to.value) ? to.value : undefined,
    order: order.value,
  })
})

function toggleOrder() {
  order.value = order.value === 'asc' ? 'desc' : 'asc'
  reload()
}

function remove(trade: Trade) {
  run(() => api.deleteTrade(trade.id), {
    confirm: `刪除 ${trade.trade_date} ${trade.code} 的交易？`,
    message: `已刪除交易 #${trade.id}`,
  })
}

function toggleForm() {
  // While editing, 新增交易 switches to a fresh create form instead of closing.
  showForm.value = editing.value ? true : !showForm.value
  editing.value = null
}

function cancelForm() {
  editing.value = null
  showForm.value = false
}

function startEdit(trade: Trade) {
  editing.value = trade
  showForm.value = true
}

function onSaved(trade: Trade) {
  message.value = `已儲存交易 #${trade.id}`
  editing.value = null
  showForm.value = false
  reload()
}

function clearFilters() {
  filterCode.value = ''
  from.value = ''
  to.value = ''
  reload()
}

onMounted(reload)
watch(() => props.market, () => {
  filterCode.value = ''
  editing.value = null
  reload()
})
</script>

<template>
  <section>
    <button type="button" class="toggle-form" @click="toggleForm">新增交易</button>
    <TradeForm
      v-if="showForm"
      :market="props.market"
      :stocks="stocks"
      :editing="editing"
      @saved="onSaved"
      @cancelled="cancelForm"
    />

    <div class="card">
      <div class="filters">
        <label>
          股票代碼
          <select v-model="filterCode" @change="reload">
            <option value="">全部</option>
            <option v-for="stock in stocks" :key="stock.id" :value="stock.code">
              {{ stock.code }}
            </option>
          </select>
        </label>
        <label>
          由
          <DateInput v-model="from" @change="reload" />
        </label>
        <label>
          至
          <DateInput v-model="to" @change="reload" />
        </label>
        <button type="button" class="secondary" @click="clearFilters">清除篩選</button>
      </div>

      <p v-if="message" class="ok">{{ message }}</p>
      <p v-if="error" class="error">{{ error?.message }}</p>

      <TradeTable
        :trades="trades"
        :stocks="stocks"
        :order="order"
        @edit="startEdit"
        @remove="remove"
        @toggle-order="toggleOrder"
      />
    </div>
  </section>
</template>

<style scoped>
.toggle-form {
  margin-bottom: 1rem;
}
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
