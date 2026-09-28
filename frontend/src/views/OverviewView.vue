<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import {
  api,
  ApiError,
  type ForecastItem,
  type ForecastItemKind,
  type ForecastMonth,
  type ForecastResponse,
  type InterestComponent,
  type OverviewResponse,
} from '../api'
import DepositForm from '../components/DepositForm.vue'
import { fmtMoney, fmtPercent, fmtPrice, signClass } from '../format'

const data = ref<OverviewResponse | null>(null)
const forecast = ref<ForecastResponse | null>(null)
const message = ref('')
const error = ref('')

// Inline amount edit for manual asset/cash rows (PATCH /api/manual-assets).
const editingId = ref<number | null>(null)
const editDraft = ref<number | ''>('')

// ----- 預測 grid (the sheet's Overview!A20:H36) -----

/** Every plan kind with its sheet row label; deposit kinds carry `return_month`
 * and get the 轉為定期 action. TBC (`other`) is rendered below the return row. */
const KIND_INFO: Record<ForecastItemKind, { label: string; deposit: boolean }> = {
  hs_deposit: { label: '定期 HS', deposit: true },
  sc_deposit: { label: 'SC高息馬拉松', deposit: true },
  interest: { label: '利息', deposit: false },
  tax: { label: 'Tax/基金/醫療保險', deposit: false },
  stock: { label: '股票', deposit: false },
  bill: { label: '繳費', deposit: false },
  other: { label: 'TBC', deposit: false },
}

/** The plan rows rendered between 定期 finish and TBC - 定期 end (sheet order). */
const PLAN_ROWS: { kind: ForecastItemKind; label: string; deposit: boolean }[] = (
  ['hs_deposit', 'sc_deposit', 'interest', 'tax', 'stock', 'bill'] as ForecastItemKind[]
).map((kind) => ({ kind, ...KIND_INFO[kind] }))

const selected = ref<{ month: string; kind: ForecastItemKind } | null>(null)
const converting = ref<ForecastItem | null>(null)
const drafts = ref<Record<number, { amount: string; note: string; return_month: string }>>({})
const newItem = reactive({ amount: '', note: '', return_month: '' })

function cellItems(month: ForecastMonth, kind: ForecastItemKind): ForecastItem[] {
  return month.plan_items.filter((item) => item.kind === kind)
}

/** Σ item amounts for a cell; null while the cell is empty (sheet: blank). */
function cellSum(month: ForecastMonth, kind: ForecastItemKind): number | null {
  const items = cellItems(month, kind)
  return items.length ? items.reduce((sum, item) => sum + item.amount, 0) : null
}

const selectedKind = computed(() =>
  selected.value ? KIND_INFO[selected.value.kind] : undefined,
)

/** The 利息 row's auto sources — a short zh label per component. */
function sourceLabel(component: InterestComponent): string {
  return component.source === 'deposit'
    ? '定期'
    : component.source === 'coupon'
      ? '債息'
      : '派息'
}
const selectedMonth = computed(() =>
  forecast.value?.months.find((month) => month.month === selected.value?.month),
)
const selectedItems = computed(() =>
  selectedMonth.value && selected.value
    ? cellItems(selectedMonth.value, selected.value.kind)
    : [],
)

function selectCell(month: string, kind: ForecastItemKind) {
  selected.value = { month, kind }
  syncDrafts()
  newItem.amount = ''
  newItem.note = ''
  newItem.return_month = ''
}

/** Rebuild the per-item drafts from the stored values (fresh after reloads). */
function syncDrafts() {
  drafts.value = {}
  for (const item of selectedItems.value) {
    drafts.value[item.id] = {
      amount: String(item.amount),
      note: item.note ?? '',
      return_month: item.return_month?.slice(0, 7) ?? '',
    }
  }
}

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

async function saveItem(item: ForecastItem) {
  const draft = drafts.value[item.id]
  if (!draft) return
  const amount = numOrNull(draft.amount)
  if (amount === null) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.updateForecastItem(item.id, {
      amount,
      note: draft.note.trim() || null,
      // `type="month"` yields YYYY-MM; empty clears the override.
      ...(item.kind === 'hs_deposit' || item.kind === 'sc_deposit'
        ? { return_month: draft.return_month || null }
        : {}),
    })
    message.value = '已更新預測'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function addItem() {
  const current = selected.value
  if (!current) return
  const amount = numOrNull(newItem.amount)
  if (amount === null) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.createForecastItem(current.month.slice(0, 7), {
      kind: current.kind,
      amount,
      note: newItem.note.trim() || undefined,
      ...(selectedKind.value?.deposit
        ? { return_month: newItem.return_month || undefined }
        : {}),
    })
    newItem.amount = ''
    newItem.note = ''
    newItem.return_month = ''
    message.value = '已新增預測項目'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeItem(item: ForecastItem) {
  if (!window.confirm(`刪除預測項目（${fmtMoney(item.amount)}）？`)) return
  try {
    await api.deleteForecastItem(item.id)
    message.value = '已刪除預測項目'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function startConvert(item: ForecastItem) {
  converting.value = item
}

function onConverted() {
  converting.value = null
  message.value = '已轉為定期'
  error.value = ''
  void load()
}

async function load() {
  try {
    const [overviewData, forecastData] = await Promise.all([api.overview(), api.forecast()])
    data.value = overviewData
    forecast.value = forecastData
    error.value = ''
    if (selected.value) syncDrafts()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function startEdit(id: number, current: number | null) {
  editingId.value = id
  editDraft.value = current ?? ''
}

async function saveEdit() {
  const id = editingId.value
  if (id === null) return
  const amount = Number(editDraft.value)
  if (String(editDraft.value).trim() === '' || !Number.isFinite(amount)) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.updateManualAsset(id, { amount })
    editingId.value = null
    message.value = '已更新'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <p v-if="error" class="error">{{ error }}</p>
  <p v-if="message" class="ok">{{ message }}</p>

  <template v-if="data">
    <div class="card">
      <div class="totals-grid">
        <div class="total-card">
          <span>總數</span>
          <strong>{{ fmtMoney(data.total_assets) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>流動資產</span>
          <strong>{{ fmtMoney(data.liquid_assets) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>流動資產 ÷ 薪金×100</span>
          <strong>{{ fmtPercent(data.liquid_ratio) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>開心 Pool</span>
          <strong>{{ fmtMoney(data.averages.pool_balance) || '—' }}</strong>
        </div>
      </div>
      <p class="muted">
        USD→HKD {{ fmtPrice(data.rate) || '—' }} · 薪金
        {{ fmtMoney(data.salary) || '—' }}
      </p>
    </div>


    <div class="overview-grid">
    <div class="card">
      <h3>資產</h3>
      <table>
        <thead>
          <tr>
            <th>項目</th>
            <th class="num">金額</th>
            <th class="num">佔比</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in data.assets" :key="row.key + row.label">
            <td>{{ row.label }}</td>
            <td
              v-if="row.manual_asset_id !== null && editingId === row.manual_asset_id"
              class="num"
            >
              <input
                v-model="editDraft"
                type="number"
                step="any"
                inputmode="decimal"
                class="cell-input"
                @keyup.enter="saveEdit"
                @keyup.esc="editingId = null"
              />
            </td>
            <td v-else class="num">{{ fmtMoney(row.amount) || '—' }}</td>
            <td class="num">{{ fmtPercent(row.share) || '—' }}</td>
            <td>
              <template v-if="row.manual_asset_id !== null">
                <template v-if="editingId === row.manual_asset_id">
                  <button type="button" class="link" @click="saveEdit">儲存</button>
                  <button type="button" class="link" @click="editingId = null">取消</button>
                </template>
                <button
                  v-else
                  type="button"
                  class="link"
                  @click="startEdit(row.manual_asset_id, row.amount)"
                >
                  編輯
                </button>
              </template>
            </td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>合計</td>
            <td class="num">{{ fmtMoney(data.assets_sum) || '—' }}</td>
            <td colspan="2"></td>
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="card">
      <h3>過去 12 個月平均</h3>
      <table>
        <tbody>
          <tr>
            <td>總數增加</td>
            <td class="num">{{ fmtMoney(data.averages.total_change) || '—' }}</td>
          </tr>
          <tr>
            <td>支出</td>
            <td class="num">{{ fmtMoney(data.averages.month_spend) || '—' }}</td>
          </tr>
          <tr>
            <td>生活支出</td>
            <td class="num">{{ fmtMoney(data.averages.living_spend) || '—' }}</td>
          </tr>
          <tr>
            <td>生活預算</td>
            <td
              class="num"
              :class="
                data.averages.living_budget === null
                  ? ''
                  : signClass(
                      data.averages.living_budget_floor - data.averages.living_budget,
                    )
              "
              :title="`低於 ${fmtMoney(data.averages.living_budget_floor)} 為綠，高於轉紅`"
            >
              {{ fmtMoney(data.averages.living_budget) || '—' }}
            </td>
          </tr>
          <tr>
            <td>存</td>
            <td class="num">{{ fmtMoney(data.averages.saved) || '—' }}</td>
          </tr>
          <tr>
            <td>利息</td>
            <td class="num">{{ fmtMoney(data.averages.interest) || '—' }}</td>
          </tr>
        </tbody>
      </table>
      <p v-if="data.averages.window_start" class="muted">
        {{ data.averages.window_start.slice(0, 7) }} ~
        {{ data.averages.window_end?.slice(0, 7) }}
      </p>
    </div>

    <div class="card">
      <h3>半流動資金</h3>
      <table>
        <tbody>
          <tr>
            <td>已定期</td>
            <td class="num">{{ fmtMoney(data.semi_liquid.deposits) || '—' }}</td>
            <td></td>
          </tr>
          <tr v-for="row in data.semi_liquid.cash_rows" :key="row.id">
            <td>{{ row.label }}</td>
            <td v-if="editingId === row.id" class="num">
              <input
                v-model="editDraft"
                type="number"
                step="any"
                inputmode="decimal"
                class="cell-input"
                @keyup.enter="saveEdit"
                @keyup.esc="editingId = null"
              />
            </td>
            <td v-else class="num">{{ fmtMoney(row.amount) || '—' }}</td>
            <td>
              <template v-if="editingId === row.id">
                <button type="button" class="link" @click="saveEdit">儲存</button>
                <button type="button" class="link" @click="editingId = null">取消</button>
              </template>
              <button
                v-else
                type="button"
                class="link"
                @click="startEdit(row.id, row.amount)"
              >
                編輯
              </button>
            </td>
          </tr>
          <tr>
            <td>活期</td>
            <td class="num">{{ fmtMoney(data.semi_liquid.cash_sum) || '—' }}</td>
            <td></td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>半流動資金</td>
            <td class="num" :class="signClass(data.semi_liquid.vs_quarter_liquid)">
              {{ fmtMoney(data.semi_liquid.total) || '—' }}
            </td>
            <td></td>
          </tr>
          <tr>
            <td colspan="3" class="num muted">
              {{ fmtPercent(data.semi_liquid.share) || '—' }} · 與25%流動相差
              <span :class="signClass(data.semi_liquid.vs_quarter_liquid)">
                {{ fmtMoney(data.semi_liquid.vs_quarter_liquid) || '—' }}
              </span>
            </td>
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="card">
      <h3>策略</h3>
      <table>
        <tbody>
          <tr>
            <td>可動用</td>
            <td
              class="num"
              :class="{ negative: (data.liquidity_tiers.can_use ?? 0) < 0 }"
            >
              {{ fmtMoney(data.liquidity_tiers.can_use) || '—' }}
            </td>
          </tr>
          <tr>
            <td>不可動用（6個月薪金）</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.cannot_use) || '—' }}</td>
          </tr>
          <tr>
            <td>短期可取回</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.short_term) || '—' }}</td>
          </tr>
          <tr>
            <td>長期可取回</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.long_term) || '—' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="card">
      <h3>IBKR</h3>
      <table>
        <tbody>
          <tr>
            <td>累計轉入 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.transferred_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>IBKR App 現值 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.now_value) || '—' }}</td>
          </tr>
          <tr>
            <td>HKD 現金</td>
            <td class="num">{{ fmtMoney(data.ibkr.hkd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>USD 現金</td>
            <td class="num">{{ fmtMoney(data.ibkr.usd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>美股總市值 (USD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.stock_value_usd) || '—' }}</td>
          </tr>
          <tr>
            <td>計算總值 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.computed_total_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>淨額 / 回報率</td>
            <td class="num" :class="signClass(data.ibkr.net)">
              {{ fmtMoney(data.ibkr.net) || '—' }} /
              {{ fmtPercent(data.ibkr.net_pct) || '—' }}
            </td>
          </tr>
          <tr>
            <td>計算 − App 差異</td>
            <td class="num" :class="signClass(data.ibkr.vs_now_value)">
              {{ fmtMoney(data.ibkr.vs_now_value) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
      <p class="muted">於 股票 → 美股 → 總覽 編輯 IBKR 數字</p>
    </div>

    <div v-if="forecast" class="card forecast">
      <h3>預測</h3>
      <div class="forecast-scroll">
        <table class="forecast-grid">
          <thead>
            <tr>
              <th></th>
              <th v-for="month in forecast.months" :key="month.month" class="num">
                {{ month.month.slice(0, 7) }}
              </th>
            </tr>
          </thead>
          <!-- Section 1 — the headline balances (sheet rows 20–24). -->
          <tbody class="s-results">
            <tr>
              <td class="row-label">ref check</td>
              <td
                v-for="month in forecast.months"
                :key="month.month"
                class="num"
                :class="signClass(month.ref_check)"
              >
                {{ fmtMoney(month.ref_check) || '—' }}
              </td>
            </tr>
            <tr>
              <td class="row-label">半流動</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.semi_liquid) || '—' }}
              </td>
            </tr>
            <tr>
              <td class="row-label">活期</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.cash) || '—' }}
              </td>
            </tr>
            <tr>
              <td class="row-label">定期 + SC</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.locked) || '—' }}
              </td>
            </tr>
          </tbody>
          <!-- Section 2 — the month's fixed inputs (sheet rows 25–27). -->
          <tbody class="s-inputs">
            <tr>
              <td class="row-label">start</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.start) || '—' }}
              </td>
            </tr>
            <tr>
              <td class="row-label">salary</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.salary) || '—' }}
              </td>
            </tr>
            <tr>
              <td class="row-label">支出</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.spend) || '—' }}
              </td>
            </tr>
          </tbody>
          <!-- Section 3 — the flow detail lines (sheet rows 28–36). -->
          <tbody class="s-flows">
            <tr>
              <td class="row-label">定期 finish</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.deposit_finish) }}
              </td>
            </tr>
            <tr v-for="row in PLAN_ROWS" :key="row.kind">
              <td class="row-label">{{ row.label }}</td>
              <td
                v-for="month in forecast.months"
                :key="month.month"
                class="num plan-cell"
                :class="{
                  selected:
                    selected?.month === month.month && selected?.kind === row.kind,
                }"
                @click="selectCell(month.month, row.kind)"
              >
                {{
                  row.kind === 'bill'
                    ? fmtMoney(month.bill) || '—'
                    : row.kind === 'interest'
                      ? fmtMoney(month.interest)
                      : fmtMoney(cellSum(month, row.kind)) || '—'
                }}
              </td>
            </tr>
            <tr>
              <td class="row-label">TBC - 定期 end</td>
              <td v-for="month in forecast.months" :key="month.month" class="num">
                {{ fmtMoney(month.deposit_return) }}
              </td>
            </tr>
            <tr>
              <td class="row-label">TBC</td>
              <td
                v-for="month in forecast.months"
                :key="month.month"
                class="num plan-cell"
                :class="{
                  selected:
                    selected?.month === month.month && selected?.kind === 'other',
                }"
                @click="selectCell(month.month, 'other')"
              >
                {{ fmtMoney(cellSum(month, 'other')) || '—' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="selected && selectedMonth && selectedKind" class="plan-editor">
        <h4>
          {{ selected.month.slice(0, 7) }} · {{ selectedKind.label }}
        </h4>
        <template v-if="selected.kind === 'interest'">
          <p class="muted">
            總額已含自動利息（到期定期利息／派息／債息）— 此處只編輯額外手動項目
          </p>
          <ul class="component-list">
            <li v-if="!selectedMonth.interest_components.length" class="muted">
              本月沒有自動利息
            </li>
            <li
              v-for="(component, index) in selectedMonth.interest_components"
              :key="index"
            >
              <span class="muted">{{ sourceLabel(component) }}</span>
              {{ component.label || '—' }}
              <span class="num">{{ fmtMoney(component.amount) || '待定' }}</span>
              <span class="muted">{{ component.received ? '已收' : '預計' }}</span>
            </li>
          </ul>
        </template>
        <p v-else-if="selected.kind === 'bill'" class="muted">
          季繳差餉預設 −{{ fmtMoney(forecast?.bill_amount) }}（1/4/7/10 月）—
          新增項目會取代該月預設值
        </p>
        <div v-for="item in selectedItems" :key="item.id" class="plan-row">
          <template v-if="drafts[item.id]">
          <input
            v-model="drafts[item.id].amount"
            type="number"
            step="any"
            inputmode="decimal"
            class="cell-input"
          />
          <input
            v-if="selectedKind.deposit"
            v-model="drafts[item.id].return_month"
            type="month"
            title="回籠月份（留空用預設 +3/+4 個月）"
          />
          <input
            v-model="drafts[item.id].note"
            type="text"
            class="note-input"
            placeholder="note"
          />
          <button type="button" class="link" @click="saveItem(item)">儲存</button>
          <button
            v-if="selectedKind.deposit"
            type="button"
            class="link"
            @click="startConvert(item)"
          >
            轉為定期
          </button>
          <button type="button" class="link" @click="removeItem(item)">刪除</button>
          </template>
        </div>
        <div class="plan-row">
          <input
            v-model="newItem.amount"
            type="number"
            step="any"
            inputmode="decimal"
            class="cell-input"
            placeholder="金額"
          />
          <input
            v-if="selectedKind.deposit"
            v-model="newItem.return_month"
            type="month"
            title="回籠月份（留空用預設 +3/+4 個月）"
          />
          <input
            v-model="newItem.note"
            type="text"
            class="note-input"
            placeholder="note"
          />
          <button type="button" class="link" @click="addItem">新增</button>
          <button type="button" class="link" @click="selected = null">收起</button>
        </div>
      </div>

      <DepositForm
        v-if="converting"
        :editing="null"
        :converting="converting"
        @saved="onConverted"
        @cancelled="converting = null"
      />
    </div>

    <div class="card invest-targets">
      <h3>投資目標</h3>
      <p class="muted">
        近3年平均 invested
        <strong class="avg">{{ fmtMoney(data.invest_targets.avg_invested) || '—' }}</strong>
      </p>
      <table>
        <thead>
          <tr>
            <th>年份</th>
            <th class="num">invested</th>
            <th class="num">目標</th>
            <th class="num">剩餘</th>
            <th class="num">增長</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in data.invest_targets.rows" :key="row.year">
            <td>{{ row.year }}</td>
            <td class="num">{{ fmtMoney(row.invested) || '—' }}</td>
            <td class="num">{{ fmtMoney(row.target) || '—' }}</td>
            <td class="num" :class="signClass(row.remain)">
              {{ fmtMoney(row.remain) || '—' }}
            </td>
            <td class="num" :class="signClass(row.growth)">
              {{ fmtPercent(row.growth) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    </div>
  </template>
</template>

<style scoped>
.overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 1rem;
  margin-bottom: 1rem;
}
.overview-grid .card {
  margin-bottom: 0;
  min-width: 0;
  overflow-x: auto;
}
.overview-grid .card.invest-targets {
  grid-column: span 2;
}
.overview-grid tfoot td.muted {
  white-space: normal;
}
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 0.5rem;
}
.total-card {
  background: var(--surface-alt);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;
}
.total-card > span {
  display: block;
  font-size: 0.8rem;
  color: var(--muted);
}
.total-card > strong {
  font-size: 1.05rem;
  font-variant-numeric: tabular-nums;
}
.cell-input {
  width: 8rem;
  text-align: right;
}
.forecast {
  overflow-x: auto;
  grid-column: 1 / -1;
}
/* The label column is sticky so it stays visible while scrolling the seven
   month columns, and reads as a header with its own background and border. */
.forecast-grid thead th:first-child {
  position: sticky;
  left: 0;
  z-index: 2;
  background: var(--surface);
  border-right: 2px solid var(--border-strong);
}
.forecast-grid .row-label {
  white-space: nowrap;
  position: sticky;
  left: 0;
  z-index: 1;
  background: var(--surface);
  border-right: 2px solid var(--border-strong);
  font-weight: 600;
}
.forecast-grid tbody.s-inputs .row-label {
  background: var(--surface-alt);
}
.forecast-grid td.num,
.forecast-grid th.num {
  min-width: 6.5rem;
}
/* Three visual sections: headline balances / fixed inputs / flow lines —
   thick separators between them, section 1 tinted like a totals block. */
.forecast-grid tbody + tbody tr:first-child td {
  border-top: 3px double var(--border-strong);
}
.forecast-grid tbody.s-results td {
  font-weight: 600;
}
.forecast-grid tbody.s-inputs td {
  background: var(--surface-alt);
}
.forecast-grid .plan-cell {
  cursor: pointer;
}
.forecast-grid .plan-cell:hover {
  background: var(--surface-alt);
}
.forecast-grid tbody.s-inputs .plan-cell:hover {
  background: var(--border);
}
.forecast-grid .plan-cell.selected {
  outline: 2px solid var(--accent, #4a7fdd);
  outline-offset: -2px;
}
.plan-editor {
  margin-top: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}
.plan-editor h4 {
  margin: 0 0 0.5rem;
}
.component-list {
  margin: 0 0 0.5rem;
  padding-left: 1.1rem;
}
.component-list li {
  display: flex;
  gap: 0.6rem;
  align-items: baseline;
}
.component-list .num {
  font-variant-numeric: tabular-nums;
}
.plan-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  margin-bottom: 0.4rem;
}
.plan-row .note-input {
  width: 14rem;
}
.forecast .deposit-form {
  margin-top: 0.75rem;
}
.avg {
  color: var(--text);
}
h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
}
</style>
