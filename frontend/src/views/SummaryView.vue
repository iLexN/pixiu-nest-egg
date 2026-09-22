<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  api,
  ApiError,
  type IbkrBlock,
  type Market,
  type SummaryResponse,
  type SummaryStock,
  type YearlySummary,
  type YearRow,
} from '../api'
import {
  compareClass,
  fmtDateTime,
  fmtFigures,
  fmtMoney,
  fmtPercent,
  fmtPrice,
  fmtShares,
  signClass,
} from '../format'

const props = defineProps<{ market: Market }>()

const summary = ref<SummaryResponse | null>(null)
const yearly = ref<YearlySummary | null>(null)
const ibkr = ref<IbkrBlock | null>(null)
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
    const [nextSummary, nextYearly, nextIbkr] = await Promise.all([
      api.summary(props.market),
      api.yearlySummary(props.market),
      props.market === 'US' ? api.ibkr() : Promise.resolve(null),
    ])
    summary.value = nextSummary
    yearly.value = nextYearly
    ibkr.value = nextIbkr
  } catch (err) {
    // Drop the previous market's data so it can't show under the wrong tab.
    summary.value = null
    yearly.value = null
    ibkr.value = null
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// Hidden stocks stay in summary.stocks so totals, rollups and reorder are
// unaffected; only the table rows are filtered.
const visibleStocks = computed(() => summary.value?.stocks.filter((s) => s.is_active) ?? [])
const hiddenCount = computed(() => (summary.value?.stocks.length ?? 0) - visibleStocks.value.length)

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

type YearField = 'invested' | 'cost' | 'market_value'
const editingYearCell = ref<string | null>(null)
const yearDraft = ref('')
const freezing = ref(false)

/** A cell counts as frozen when the snapshot carries a value for it. */
function frozen(row: YearRow, field: YearField): boolean {
  return row.snapshot?.[field] !== null && row.snapshot?.[field] !== undefined
}

function frozenTitle(row: YearRow): string {
  return row.snapshot ? `已凍結 ${fmtDateTime(row.snapshot.updated_at)}` : ''
}

function startYearEdit(row: YearRow, field: YearField) {
  editingYearCell.value = `${row.year}:${field}`
  const current = row.snapshot?.[field] ?? row[field]
  yearDraft.value = current === null ? '' : String(current)
}

async function saveYearEdit(row: YearRow, field: YearField) {
  const text = String(yearDraft.value).trim()
  const parsed = text === '' ? null : Number(text)
  if (parsed !== null && (!Number.isFinite(parsed) || parsed < 0)) {
    error.value = '數值必須是非負數字'
    return
  }
  try {
    await api.updateYearly(props.market, row.year, { [field]: parsed })
    editingYearCell.value = null
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function freezeYear(row: YearRow) {
  if (
    row.snapshot &&
    !window.confirm(`${row.year} 已有凍結數值，確定用現在的計算值覆蓋？`)
  ) {
    return
  }
  freezing.value = true
  try {
    await api.freezeYearly(props.market, row.year)
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  } finally {
    freezing.value = false
  }
}

// The 美股 sheet's A1:B5 block — four manual inputs; the derived figures
// (computed total, net vs transferred) come back in the response.
const editingIbkr = ref(false)
const ibkrDraft = ref({
  transferred_delta: '' as number | '',
  now_value: '' as number | '',
  hkd_cash: '' as number | '',
  usd_cash: '' as number | '',
})

// The 轉入 input is a delta; this previews the new cumulative total.
const ibkrTransferredPreview = computed(() => {
  const text = String(ibkrDraft.value.transferred_delta).trim()
  if (text === '') return null
  const delta = Number(text)
  if (!Number.isFinite(delta)) return null
  return (ibkr.value?.transferred_hkd ?? 0) + delta
})

function startIbkrEdit() {
  ibkrDraft.value = {
    transferred_delta: '',
    now_value: ibkr.value?.now_value ?? '',
    hkd_cash: ibkr.value?.hkd_cash ?? '',
    usd_cash: ibkr.value?.usd_cash ?? '',
  }
  editingIbkr.value = true
}

function ibkrField(raw: number | '', label: string): number | null {
  if (String(raw).trim() === '') return null
  const parsed = Number(raw)
  if (!Number.isFinite(parsed) || parsed < 0) {
    throw new Error(`${label}必須是非負數字`)
  }
  return parsed
}

async function saveIbkr() {
  try {
    ibkr.value = await api.updateIbkr({
      // Undefined leaves the stored total untouched; a delta adds to it.
      transferred_hkd: ibkrTransferredPreview.value ?? undefined,
      now_value: ibkrField(ibkrDraft.value.now_value, 'IBKR App 現值'),
      hkd_cash: ibkrField(ibkrDraft.value.hkd_cash, 'HKD 現金'),
      usd_cash: ibkrField(ibkrDraft.value.usd_cash, 'USD 現金'),
    })
    editingIbkr.value = false
    error.value = ''
  } catch (err) {
    error.value = err instanceof ApiError || err instanceof Error ? err.message : String(err)
  }
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
      <div class="total-card">
        <span>上月</span>
        <strong>
          <template v-if="summary.last_month">
            <span
              :class="compareClass(summary.totals.net_percent, summary.last_month.percent)"
              >{{ fmtPercent(summary.last_month.percent) || '—' }}</span
            >
            /
            <span
              :class="compareClass(summary.totals.net_amount, summary.last_month.amount)"
              >{{ fmtMoney(summary.last_month.amount) }}</span
            >
          </template>
          <template v-else>—</template>
        </strong>
      </div>
      <div class="total-card">
        <span>最高</span>
        <strong>
          <template v-if="summary.max">{{ fmtFigures(summary.max.percent, summary.max.amount) }}</template>
          <template v-else>—</template>
        </strong>
      </div>
      <div class="total-card">
        <span>累計派息</span>
        <strong>{{ fmtMoney(summary.totals.dividends_received) }}</strong>
      </div>
      <div class="total-card">
        <span>累計派息%</span>
        <strong :class="signClass(summary.totals.dividend_return)">
          {{ fmtPercent(summary.totals.dividend_return) }}
        </strong>
      </div>
      <div class="total-card">
        <span>淨投入總本金</span>
        <strong>{{ fmtMoney(summary.totals.net_invested) }}</strong>
      </div>
      <div class="total-card">
        <span>實質動態總回報%</span>
        <strong :class="signClass(summary.totals.real_total_return)">
          {{ fmtPercent(summary.totals.real_total_return) }}
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
          <th class="num" :title="summary.average_price_definition">平均單價</th>
          <th class="num">成本</th>
          <th class="num">現價</th>
          <th class="num">當前總市值</th>
          <th class="num">未實現金額</th>
          <th class="num">未實現報酬率</th>
          <th class="num">累計派息</th>
          <th class="num">累計派息%</th>
          <th class="num">淨投入總本金</th>
          <th class="num">淨攤薄單價</th>
          <th class="num">實質動態總回報%</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="stock in visibleStocks"
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
          <td class="num">{{ fmtMoney(stock.dividends_received) }}</td>
          <td class="num" :class="signClass(stock.dividend_return)">
            {{ fmtPercent(stock.dividend_return) }}
          </td>
          <td class="num">{{ fmtMoney(stock.net_invested) }}</td>
          <td class="num">{{ fmtPrice(stock.net_diluted_price) }}</td>
          <td class="num" :class="signClass(stock.real_total_return)">
            {{ fmtPercent(stock.real_total_return) }}
          </td>
        </tr>
      </tbody>
    </table>

    <p v-if="hiddenCount > 0" class="muted">
      已隱藏 {{ hiddenCount }} 支股票 — 在股票管理中可重新顯示（其數值仍計入合計）
    </p>

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
          <th class="num">成本</th>
          <th class="num">當前總市值</th>
          <th class="num">佔市值</th>
          <th class="num">未實現報酬率</th>
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

    <template v-if="yearly">
      <h4>每年總覽</h4>
      <p class="muted">
        年末成本與總市值為凍結值（*），年底按「凍結」或點擊儲存格輸入；其餘由交易與派息即時計算。
      </p>
      <table>
        <thead>
          <tr>
            <th>年份</th>
            <th class="num">net invested</th>
            <th class="num">成本</th>
            <th class="num">總市值</th>
            <th class="num">派息 ÷ 成本</th>
            <th class="num">派息 ÷ 市值</th>
            <th class="num">派息</th>
            <th class="num">月均派息</th>
            <th class="num">派息 YoY</th>
            <th class="num">invested YoY</th>
            <th aria-label="凍結"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in yearly.years" :key="row.year">
            <td>{{ row.year }}</td>
            <td
              class="num year-cell"
              :class="{ frozen: frozen(row, 'invested') }"
              :title="frozen(row, 'invested') ? frozenTitle(row) : ''"
            >
              <template v-if="editingYearCell === `${row.year}:invested`">
                <input
                  v-model="yearDraft"
                  type="number"
                  step="any"
                  @keyup.enter="saveYearEdit(row, 'invested')"
                />
                <button type="button" class="link" @click="saveYearEdit(row, 'invested')">
                  儲存
                </button>
                <button type="button" class="link" @click="editingYearCell = null">取消</button>
              </template>
              <button v-else type="button" class="link" @click="startYearEdit(row, 'invested')">
                {{ fmtMoney(row.invested) || '輸入' }}{{ frozen(row, 'invested') ? '*' : '' }}
              </button>
            </td>
            <td
              v-for="field in ['cost', 'market_value'] as const"
              :key="field"
              class="num year-cell"
              :class="{ frozen: frozen(row, field) }"
              :title="frozen(row, field) ? frozenTitle(row) : ''"
            >
              <template v-if="editingYearCell === `${row.year}:${field}`">
                <input
                  v-model="yearDraft"
                  type="number"
                  step="any"
                  @keyup.enter="saveYearEdit(row, field)"
                />
                <button type="button" class="link" @click="saveYearEdit(row, field)">儲存</button>
                <button type="button" class="link" @click="editingYearCell = null">取消</button>
              </template>
              <button v-else type="button" class="link" @click="startYearEdit(row, field)">
                {{ fmtMoney(row[field]) || '輸入' }}{{ frozen(row, field) ? '*' : '' }}
              </button>
            </td>
            <td class="num" :class="signClass(row.yield_on_cost)">
              {{ row.yield_on_cost === null ? '—' : fmtPercent(row.yield_on_cost) }}
            </td>
            <td class="num" :class="signClass(row.yield_on_value)">
              {{ row.yield_on_value === null ? '—' : fmtPercent(row.yield_on_value) }}
            </td>
            <td class="num">{{ fmtMoney(row.dividends) }}</td>
            <td class="num">{{ fmtMoney(row.monthly_dividend) }}</td>
            <td class="num" :class="signClass(row.dividend_yoy)">
              {{ row.dividend_yoy === null ? '—' : fmtPercent(row.dividend_yoy) }}
            </td>
            <td class="num" :class="signClass(row.invested_yoy)">
              {{ row.invested_yoy === null ? '—' : fmtPercent(row.invested_yoy) }}
            </td>
            <td>
              <button
                v-if="row.year === Number(yearly.today.slice(0, 4))"
                type="button"
                class="link"
                :disabled="freezing"
                title="把現在的成本與總市值存為今年的凍結值"
                @click="freezeYear(row)"
              >
                凍結
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </template>
  </section>

  <section v-if="market === 'US' && ibkr" class="card">
    <h3>IBKR</h3>
    <form v-if="editingIbkr" class="ibkr-form" @submit.prevent="saveIbkr">
      <label>
        轉入 (HKD)
        <input
          v-model="ibkrDraft.transferred_delta"
          type="number"
          step="any"
          inputmode="decimal"
          placeholder="+金額"
        />
      </label>
      <label>
        IBKR App 現值 (HKD)
        <input v-model="ibkrDraft.now_value" type="number" step="any" inputmode="decimal" />
      </label>
      <label>
        HKD 現金
        <input v-model="ibkrDraft.hkd_cash" type="number" step="any" inputmode="decimal" />
      </label>
      <label>
        USD 現金
        <input v-model="ibkrDraft.usd_cash" type="number" step="any" inputmode="decimal" />
      </label>
      <div class="ibkr-actions">
        <button type="submit">儲存</button>
        <button type="button" class="link" @click="editingIbkr = false">取消</button>
      </div>
      <p class="muted ibkr-preview">
        累計轉入 {{ fmtMoney(ibkr?.transferred_hkd) || '0' }}
        <template v-if="ibkrTransferredPreview !== null">
          → <strong>{{ fmtMoney(ibkrTransferredPreview) }}</strong>
        </template>
      </p>
    </form>
    <template v-else>
      <table>
        <tbody>
          <tr>
            <td>累計轉入 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.transferred_hkd) || '—' }}</td>
            <td>IBKR App 現值 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.now_value) || '—' }}</td>
          </tr>
          <tr>
            <td>HKD 現金</td>
            <td class="num">{{ fmtMoney(ibkr.hkd_cash) || '—' }}</td>
            <td>USD 現金</td>
            <td class="num">{{ fmtMoney(ibkr.usd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>美股總市值 (USD)</td>
            <td class="num">{{ fmtMoney(ibkr.stock_value_usd) || '—' }}</td>
            <td>計算總值 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.computed_total_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>淨額 / 回報率</td>
            <td class="num" :class="signClass(ibkr.net)">
              {{ fmtMoney(ibkr.net) || '—' }} / {{ fmtPercent(ibkr.net_pct) || '—' }}
            </td>
            <td>計算 − App 差異</td>
            <td class="num" :class="signClass(ibkr.vs_now_value)">
              {{ fmtMoney(ibkr.vs_now_value) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
      <button type="button" class="link" @click="startIbkrEdit">編輯</button>
    </template>
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
  grid-template-columns: repeat(4, 1fr);
  gap: 0.75rem;
  margin: 1rem 0;
}
.total-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.75rem;
}
.total-card > span {
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
.year-cell input {
  width: 7rem;
}
.year-cell.frozen .link {
  text-decoration: underline dotted;
}
.ibkr-form {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
}
.ibkr-form label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.2rem;
}
.ibkr-form input {
  width: 9rem;
}
.ibkr-actions {
  display: flex;
  gap: 0.5rem;
}
.ibkr-preview {
  flex-basis: 100%;
  margin: 0;
}
h4 {
  margin: 1.5rem 0 0.5rem;
}
</style>
