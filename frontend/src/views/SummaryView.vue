<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { api, ApiError, type Market, type SummaryResponse, type SummaryStock } from '../api'
import { fmtMoney, fmtPercent, fmtPrice, fmtShares, signClass } from '../format'

const props = defineProps<{ market: Market }>()

const summary = ref<SummaryResponse | null>(null)
const error = ref('')
const editingPrice = ref<number | null>(null)
const priceDraft = ref('')
const draggingId = ref<number | null>(null)
const dragOverId = ref<number | null>(null)
const savingOrder = ref(false)
const priceFileInput = ref<HTMLInputElement | null>(null)
const uploadingPrices = ref(false)
const uploadMessage = ref('')

async function load() {
  error.value = ''
  try {
    summary.value = await api.summary(props.market)
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function priceClass(stock: SummaryStock): string {
  if (stock.current_price === null || stock.weighted_avg_buy_price <= 0) return ''
  return signClass(stock.current_price - stock.weighted_avg_buy_price)
}

function startEdit(id: number, current: number | null) {
  editingPrice.value = id
  priceDraft.value = current === null ? '' : String(current)
}

async function savePrice(id: number) {
  const text = String(priceDraft.value).trim()
  const parsed = Number(text)
  if (text === '' || !Number.isFinite(parsed)) {
    error.value = '現價必須是數字'
    return
  }
  try {
    await api.updateStock(id, { manual_price: parsed })
    editingPrice.value = null
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function onDragStart(id: number, event: DragEvent) {
  draggingId.value = id
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', String(id))
  }
}

function onDragOver(id: number, event: DragEvent) {
  if (draggingId.value === null || draggingId.value === id) return
  event.preventDefault()
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'
  dragOverId.value = id
}

async function onDrop(targetId: number, event: DragEvent) {
  event.preventDefault()
  const sourceId = draggingId.value
  draggingId.value = null
  dragOverId.value = null
  if (!summary.value || sourceId === null || sourceId === targetId) return

  const rows = [...summary.value.stocks]
  const from = rows.findIndex((stock) => stock.id === sourceId)
  const to = rows.findIndex((stock) => stock.id === targetId)
  if (from < 0 || to < 0) return

  const [moved] = rows.splice(from, 1)
  rows.splice(to, 0, moved)
  summary.value.stocks = rows

  savingOrder.value = true
  error.value = ''
  try {
    await api.reorderStocks(props.market, rows.map((stock) => stock.id))
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
    await load()
  } finally {
    savingOrder.value = false
  }
}

async function onPriceFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return

  uploadingPrices.value = true
  error.value = ''
  uploadMessage.value = ''
  try {
    const report = await api.uploadPrices(await file.text())
    const parts = [`已更新 ${report.updated.length} 支股票現價`]
    if (report.unmatched.length > 0) {
      parts.push(`找不到股票：${report.unmatched.join('、')}`)
    }
    if (report.invalid.length > 0) {
      parts.push(`無效項目 ${report.invalid.length} 筆`)
    }
    if (report.not_updated.length > 0) {
      parts.push(`未更新：${report.not_updated.join('、')}`)
    }
    uploadMessage.value = parts.join('；')
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  } finally {
    uploadingPrices.value = false
  }
}

function onDragEnd() {
  draggingId.value = null
  dragOverId.value = null
}

onMounted(load)
watch(() => props.market, load)
</script>

<template>
  <section v-if="summary" class="card">
    <h3>{{ props.market === 'HK' ? '港股' : '美股' }}持倉總覽</h3>
    <div class="summary-actions">
      <button type="button" :disabled="uploadingPrices" @click="priceFileInput?.click()">
        匯入現價 JSON
      </button>
      <input
        ref="priceFileInput"
        type="file"
        accept=".json,application/json"
        hidden
        @change="onPriceFile"
      />
      <span v-if="uploadMessage" class="muted">{{ uploadMessage }}</span>
    </div>
    <p v-if="error" class="error">{{ error }}</p>

    <div class="totals-grid" aria-label="市場合計">
      <div class="total-card">
        <span>總買入成本</span>
        <strong>{{ fmtMoney(summary.totals.buy_cost) }}</strong>
      </div>
      <div class="total-card">
        <span>當前總市值</span>
        <strong>{{ fmtMoney(summary.totals.market_value) }}</strong>
      </div>
      <div class="total-card">
        <span>未實現金額</span>
        <strong :class="signClass(summary.totals.net_amount)">
          {{ fmtMoney(summary.totals.net_amount) }}
        </strong>
      </div>
      <div class="total-card">
        <span>未實現報酬率</span>
        <strong :class="signClass(summary.totals.net_percent)">
          {{ fmtPercent(summary.totals.net_percent) }}
        </strong>
      </div>
    </div>

    <table>
      <thead>
        <tr>
          <th aria-label="Reorder"></th>
          <th>類別</th>
          <th>股票代碼</th>
          <th>Stock</th>
          <th class="num">股數</th>
          <th class="num" :title="summary.average_price_definition">加權平均買入單價</th>
          <th class="num">總買入成本</th>
          <th class="num">現價</th>
          <th class="num">當前總市值</th>
          <th class="num">未實現金額</th>
          <th class="num">未實現報酬率</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="stock in summary.stocks"
          :key="stock.id"
          :class="{ 'drag-over': dragOverId === stock.id, dragging: draggingId === stock.id }"
          @dragover="onDragOver(stock.id, $event)"
          @drop="onDrop(stock.id, $event)"
        >
          <td
            class="drag-handle"
            draggable="true"
            title="拖曳以調整次序"
            @dragstart="onDragStart(stock.id, $event)"
            @dragend="onDragEnd"
          >
            ↕
          </td>
          <td>{{ stock.sector ?? '' }}</td>
          <td>{{ stock.code }}</td>
          <td>{{ stock.ticker ?? '' }}</td>
          <td class="num">{{ fmtShares(stock.shares_held) }}</td>
          <td class="num">{{ fmtPrice(stock.weighted_avg_buy_price) }}</td>
          <td class="num">{{ fmtMoney(stock.total_buy_cost) }}</td>
          <td class="num price-cell">
            <template v-if="editingPrice === stock.id">
              <input
                v-model="priceDraft"
                type="number"
                step="any"
                @keyup.enter="savePrice(stock.id)"
              />
              <button type="button" class="link" @click="savePrice(stock.id)">儲存</button>
              <button type="button" class="link" @click="editingPrice = null">取消</button>
            </template>
            <button
              v-else
              type="button"
              class="link"
              :class="priceClass(stock)"
              @click="startEdit(stock.id, stock.current_price)"
            >
              {{ stock.current_price === null ? '設定現價' : fmtPrice(stock.current_price) }}
            </button>
          </td>
          <td class="num">{{ fmtMoney(stock.market_value) }}</td>
          <td class="num" :class="signClass(stock.unrealized_amount)">
            {{ fmtMoney(stock.unrealized_amount) }}
          </td>
          <td class="num" :class="signClass(stock.unrealized_return)">
            {{ fmtPercent(stock.unrealized_return) }}
          </td>
        </tr>
      </tbody>
    </table>

    <p v-if="summary.totals.excluded_codes.length > 0" class="muted">
      未計入市值與淨額（未輸入現價或持倉為 0）：{{ summary.totals.excluded_codes.join('、') }}
      （成本已計入「總買入成本」，比較用成本為
      {{ fmtMoney(summary.totals.buy_cost_priced) }}）
    </p>

    <h4>行業分佈</h4>
    <table>
      <thead>
        <tr>
          <th>類別</th>
          <th class="num">buy total</th>
          <th class="num">now total</th>
          <th class="num">佔市值</th>
          <th class="num">% change</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="sector in summary.sectors" :key="sector.sector">
          <td>{{ sector.sector }}</td>
          <td class="num">{{ fmtMoney(sector.buy_cost) }}</td>
          <td class="num">{{ fmtMoney(sector.market_value) }}</td>
          <td class="num">{{ fmtPercent(sector.share_of_market_value) }}</td>
          <td class="num" :class="signClass(sector.percent_change)">
            {{ fmtPercent(sector.percent_change) }}
          </td>
        </tr>
        <tr v-if="summary.sectors.length === 0">
          <td colspan="5" class="muted">尚未有持倉</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.summary-actions {
  align-items: center;
  display: flex;
  gap: 0.75rem;
  margin: 0.5rem 0;
}
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
  margin: 1rem 0;
}
.total-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.75rem;
}
.total-card span {
  color: var(--muted);
  display: block;
  font-size: 0.78rem;
  margin-bottom: 0.25rem;
}
.total-card strong {
  font-size: 1.05rem;
  font-weight: 650;
}
.price-cell input {
  width: 6rem;
}
.price-cell .link.positive {
  color: var(--positive);
}
.price-cell .link.negative {
  color: var(--negative);
}
.drag-handle {
  color: var(--muted);
  cursor: grab;
  text-align: center;
  user-select: none;
  width: 2rem;
}
.drag-handle:active {
  cursor: grabbing;
}
.dragging {
  opacity: 0.45;
}
.drag-over td {
  border-top: 2px solid var(--highlight);
}
h4 {
  margin: 1.5rem 0 0.5rem;
}
</style>
