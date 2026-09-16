<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import DividendForm from '../components/DividendForm.vue'
import DividendReceiveForm from '../components/DividendReceiveForm.vue'
import DividendTable from '../components/DividendTable.vue'
import {
  api,
  ApiError,
  type Dividend,
  type DividendSummary,
  type Market,
  type SummaryStock,
} from '../api'
import { fmtMoney, fmtPercent } from '../format'

const props = defineProps<{ market: Market }>()

const summary = ref<DividendSummary | null>(null)
const stocks = ref<SummaryStock[]>([])
const received = ref<Dividend[]>([])
const year = ref<number | null>(null)
const filterStockId = ref<number | null>(null)
const message = ref('')
const error = ref('')
const editing = ref<Dividend | null>(null)
const receiving = ref<Dividend | null>(null)
const showForm = ref(false)

async function load() {
  error.value = ''
  try {
    const [dividendSummary, marketSummary] = await Promise.all([
      api.dividendSummary(props.market),
      api.summary(props.market),
    ])
    summary.value = dividendSummary
    stocks.value = marketSummary.stocks
    received.value = await api.listDividends({
      market: props.market,
      status: 'received',
      stock_id: filterStockId.value ?? undefined,
      year: year.value ?? undefined,
      order: 'desc',
    })
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function remove(dividend: Dividend) {
  if (!window.confirm(`刪除 ${dividend.code}（${dividend.pay_date}）派息記錄？`)) return
  try {
    await api.deleteDividend(dividend.id)
    message.value = `已刪除 ${dividend.code} 派息記錄`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function toggleForm() {
  // While editing, 記錄派息 switches to a fresh create form instead of closing.
  showForm.value = editing.value ? true : !showForm.value
  editing.value = null
}

function cancelForm() {
  editing.value = null
  showForm.value = false
}

function startEdit(dividend: Dividend) {
  editing.value = dividend
  receiving.value = null
  showForm.value = true
}

function onSaved() {
  message.value = '已儲存派息記錄'
  error.value = ''
  editing.value = null
  receiving.value = null
  showForm.value = false
  void load()
}

const yearOptions = computed(() => summary.value?.history_years ?? [])

// YoY change vs the previous row; years are sorted ascending.
function yearChange(index: number): number | null {
  const years = summary.value?.years
  const prev = index > 0 ? years?.[index - 1].total : undefined
  if (years === undefined || prev === undefined || prev === 0) return null
  return (years[index].total - prev) / prev
}

watch(() => props.market, () => {
  editing.value = null
  receiving.value = null
  showForm.value = false
  year.value = null
  filterStockId.value = null
  void load()
})
watch([year, filterStockId], () => void load())

onMounted(load)
</script>

<template>
  <section>
    <button type="button" class="toggle-form" @click="toggleForm">記錄派息</button>
    <DividendReceiveForm
      v-if="receiving"
      :dividend="receiving"
      @saved="onSaved"
      @cancelled="receiving = null"
    />
    <DividendForm
      v-else-if="showForm"
      :stocks="stocks"
      :editing="editing"
      @saved="onSaved"
      @cancelled="cancelForm"
    />

    <div v-if="summary" class="card">
      <h3>待收派息</h3>
      <DividendTable
        :dividends="summary.pending"
        receivable
        empty-text="沒有待收派息"
        @receive="receiving = $event"
        @edit="startEdit"
        @remove="remove"
      />
    </div>

    <div v-if="summary" class="card">
      <h3>
        已收記錄
        <select v-model="filterStockId" class="year-filter">
          <option :value="null">全部股票</option>
          <option v-for="stock in stocks" :key="stock.id" :value="stock.id">
            {{ stock.code }}
          </option>
        </select>
        <select v-model="year" class="year-filter">
          <option :value="null">全部年份</option>
          <option v-for="y in yearOptions" :key="y" :value="y">{{ y }}</option>
        </select>
      </h3>
      <DividendTable
        :dividends="received"
        empty-text="沒有已收派息記錄"
        @edit="startEdit"
        @remove="remove"
      />
    </div>

    <div v-if="summary && summary.years.length > 0" class="card">
      <h3>年度合計（實收）</h3>
      <table class="rollup">
        <thead>
          <tr>
            <th>年份</th>
            <th class="num">實收合計</th>
            <th class="num">月均</th>
            <th class="num">按年變化</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(rollup, index) in summary.years" :key="rollup.year">
            <td>{{ rollup.year }}</td>
            <td class="num">{{ fmtMoney(rollup.total) }}</td>
            <td class="num">{{ fmtMoney(rollup.total / 12) }}</td>
            <td class="num">
              {{ yearChange(index) === null ? '—' : fmtPercent(yearChange(index)) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.toggle-form {
  margin-bottom: 1rem;
}
h3 {
  margin: 0 0 0.75rem;
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.year-filter {
  font-size: 0.8rem;
  font-weight: normal;
}
.card + .card {
  margin-top: 1rem;
}
.rollup {
  table-layout: fixed;
}
.rollup th.num {
  width: 10rem;
}
</style>
