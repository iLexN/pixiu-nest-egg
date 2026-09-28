<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  api,
  ApiError,
  type ManualAsset,
  type ManualAssetKind,
  type ManualAssetLiquidity,
  type MonthDetail,
  type MonthItem,
  type MonthItemCategory,
  type MonthSettings,
  type MonthStat,
  type MonthStatPatch,
  type MonthSuggestion,
  type MonthSummary,
  type NewManualAsset,
} from '../api'
import { fmtMoney, fmtPercent, signClass, todayIso } from '../format'

const summary = ref<MonthSummary | null>(null)
const months = ref<MonthStat[]>([])
const detail = ref<MonthDetail | null>(null)
const settings = ref<MonthSettings | null>(null)
const assets = ref<ManualAsset[]>([])
const message = ref('')
const error = ref('')

const currentYear = new Date().getFullYear()
const year = ref(currentYear)
const selectedMonth = ref<string | null>(null)

// History rows never derive live (the backend keeps their NULL totals as
// absent), so re-snapshotting or unfreezing only applies from this month on.
const selectedIsCurrentOrLater = computed(
  () => !!selectedMonth.value && selectedMonth.value.slice(0, 7) >= todayIso().slice(0, 7),
)

const yearOptions = computed(() => {
  const set = new Set<number>([currentYear])
  for (const y of summary.value?.years ?? []) set.add(y.year)
  return [...set].sort((a, b) => a - b)
})

/** The sheet's A1:N4 yearly block — last 3 years. */
const recentYears = computed(() => (summary.value?.years ?? []).slice(-3))

const CATEGORY_LABELS: Record<MonthItemCategory, string> = {
  adjustment: '調整',
  extra_spend: '額外支出',
  income: '額外收入',
  interest: '利息',
  entertainment: '娛樂支出',
}
// Sheet column order: G 調整, I/J 額外支出, L 額外收入, N 利息, O 娛樂支出.
const CATEGORY_ORDER: MonthItemCategory[] = [
  'adjustment',
  'extra_spend',
  'income',
  'interest',
  'entertainment',
]

/** Auto interest component sources (定期 end / received coupon / HK 派息). */
const INTEREST_SOURCE_LABELS: Record<string, string> = {
  deposit: '定期',
  coupon: '券息',
  dividend: '派息',
}
const interestAuto = computed(() => detail.value?.interest_auto ?? [])

function itemsOf(category: MonthItemCategory): MonthItem[] {
  return (detail.value?.items ?? []).filter((item) => item.category === category)
}

function suggestionsOf(category: MonthItemCategory): MonthSuggestion[] {
  return (detail.value?.suggestions ?? []).filter((s) => s.category === category)
}

/** 生活按年: spending less than last year is green, >2% inflation is red. */
function yoyClass(value: number | null): string {
  if (value === null) return ''
  if (value < 0) return 'positive'
  if (value > 0.02) return 'negative'
  return ''
}

/** The imported month's G total already folded events into one adjustment. */
const hasImportedTotal = computed(() =>
  (detail.value?.items ?? []).some(
    (item) => item.auto_key === null && item.note?.startsWith('='),
  ),
)

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

function fmtMonth(month: string): string {
  return month.slice(0, 7)
}

async function load() {
  error.value = ''
  try {
    const [summaryData, monthsData, allMonths, settingsData, assetsData] = await Promise.all([
      api.monthSummary(),
      api.listMonths(year.value),
      api.listMonths(),
      api.monthSettings(),
      api.listManualAssets(),
    ])
    summary.value = summaryData
    months.value = monthsData
    latestStoredMonth.value = allMonths.at(-1)?.month ?? null
    settings.value = settingsData
    assets.value = assetsData
    if (!newMonth.value) newMonth.value = defaultNewMonth()
    if (selectedMonth.value) {
      detail.value = await api.monthDetail(selectedMonth.value)
    }
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function selectYear() {
  try {
    months.value = await api.listMonths(year.value)
    if (selectedMonth.value && !selectedMonth.value.startsWith(`${year.value}-`)) {
      selectedMonth.value = null
      detail.value = null
    }
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function selectMonth(month: MonthStat) {
  selectedMonth.value = month.month
  error.value = ''
  try {
    detail.value = await api.monthDetail(month.month)
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
  startEdit()
}

// ----- 新增月份 -----

const newMonth = ref('')
const latestStoredMonth = ref<string | null>(null)

/** Default: the month after the latest stored row, else the current month. */
function defaultNewMonth(): string {
  const latest = latestStoredMonth.value ?? `${todayIso().slice(0, 7)}-01`
  const date = new Date(`${latest.slice(0, 7)}-01T00:00:00`)
  date.setMonth(date.getMonth() + 1)
  return `${date.getFullYear()}-${`${date.getMonth() + 1}`.padStart(2, '0')}`
}

async function addMonth() {
  const ym = newMonth.value || defaultNewMonth()
  try {
    await api.patchMonth(ym, {})
    message.value = `已新增 ${ym}`
    error.value = ''
    newMonth.value = ''
    await load()
    const created = months.value.find((m) => m.month === `${ym}-01`)
    if (created) await selectMonth(created)
    else {
      selectedMonth.value = `${ym}-01`
      detail.value = await api.monthDetail(selectedMonth.value)
      startEdit()
    }
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- month editor -----

interface MonthDraft {
  start_cash: string | number
  salary: string | number
  end_cash_override: string | number
  pool_input: string | number
  note: string
}

const monthDraft = ref<MonthDraft>({
  start_cash: '',
  salary: '',
  end_cash_override: '',
  pool_input: '',
  note: '',
})

function startEdit() {
  const month = detail.value?.month
  monthDraft.value = {
    start_cash: month?.start_cash ?? '',
    salary: month?.salary ?? '',
    end_cash_override: month?.end_cash_override ?? '',
    pool_input: month?.pool_input ?? '',
    note: month?.note ?? '',
  }
}

async function saveMonth() {
  if (!selectedMonth.value) return
  const patch: MonthStatPatch = {
    start_cash: numOrNull(monthDraft.value.start_cash),
    salary: numOrNull(monthDraft.value.salary),
    end_cash_override: numOrNull(monthDraft.value.end_cash_override),
    pool_input: numOrNull(monthDraft.value.pool_input) ?? 0,
    note: monthDraft.value.note.trim() || null,
  }
  try {
    await api.patchMonth(selectedMonth.value, patch)
    message.value = '已儲存月份'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function recapture() {
  if (!selectedMonth.value) return
  try {
    await api.patchMonth(selectedMonth.value, { recapture: true })
    message.value = '已重新擷取月初及即時總數'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function goLive() {
  if (!selectedMonth.value) return
  try {
    await api.patchMonth(selectedMonth.value, { total_assets: null, liquid_assets: null })
    message.value = '已改為即時計算'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeMonth() {
  const month = selectedMonth.value
  if (!month) return
  if (!window.confirm(`刪除 ${fmtMonth(month)}？其項目會一併刪除。`)) return
  try {
    await api.deleteMonth(month)
    selectedMonth.value = null
    detail.value = null
    message.value = `已刪除 ${fmtMonth(month)}`
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- items -----

const editingItem = ref<MonthItem | null>(null)
const itemDraft = ref<{
  label: string
  amount: string | number
  exclude_from_living: boolean
  note: string
}>({
  label: '',
  amount: '',
  exclude_from_living: false,
  note: '',
})

function startItemEdit(item: MonthItem) {
  editingItem.value = item
  itemDraft.value = {
    label: item.label ?? '',
    amount: item.amount,
    exclude_from_living: item.exclude_from_living,
    note: item.note ?? '',
  }
}

async function saveItem() {
  const item = editingItem.value
  if (!item) return
  const amount = numOrNull(itemDraft.value.amount)
  if (amount === null) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.updateMonthItem(item.id, {
      label: itemDraft.value.label.trim() || null,
      amount,
      exclude_from_living:
        item.category === 'entertainment' ? itemDraft.value.exclude_from_living : false,
      note: itemDraft.value.note.trim() || null,
    })
    editingItem.value = null
    message.value = '已儲存項目'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeItem(item: MonthItem) {
  if (!window.confirm(`刪除項目 ${item.label ?? item.amount}？`)) return
  try {
    await api.deleteMonthItem(item.id)
    message.value = '已刪除項目'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

const newItem = ref<{
  category: MonthItemCategory
  label: string
  amount: string | number
  exclude_from_living: boolean
  note: string
}>({
  category: 'adjustment',
  label: '',
  amount: '',
  exclude_from_living: false,
  note: '',
})

async function addItem() {
  if (!selectedMonth.value) return
  const amount = numOrNull(newItem.value.amount)
  if (amount === null) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.createMonthItem(selectedMonth.value, {
      category: newItem.value.category,
      label: newItem.value.label.trim() || null,
      amount,
      exclude_from_living:
        newItem.value.category === 'entertainment' ? newItem.value.exclude_from_living : false,
      note: newItem.value.note.trim() || null,
    })
    newItem.value = {
      category: 'adjustment',
      label: '',
      amount: '',
      exclude_from_living: false,
      note: '',
    }
    message.value = '已新增項目'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function acceptSuggestion(suggestion: MonthSuggestion) {
  if (!selectedMonth.value) return
  try {
    await api.createMonthItem(selectedMonth.value, {
      category: suggestion.category,
      label: suggestion.label,
      amount: suggestion.amount,
      auto_key: suggestion.auto_key,
    })
    message.value = '已接受建議'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function dismissSuggestion(suggestion: MonthSuggestion) {
  if (!selectedMonth.value) return
  try {
    await api.dismissMonthItem(selectedMonth.value, suggestion.auto_key)
    message.value = '已忽略建議'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- settings -----

const settingsOpen = ref(false)
const salaryDraft = ref<string | number>('')
const rateDraft = ref<string | number>('')
const billDraft = ref<string | number>('')
const editingSettings = ref(false)

function startSettingsEdit() {
  salaryDraft.value = settings.value?.salary ?? ''
  // Stored as a fraction; edited as a percent, same as deposit rates.
  const rate = summary.value?.pool_rate
  rateDraft.value = rate === null || rate === undefined ? '' : rate * 100
  billDraft.value = settings.value?.bill_amount ?? ''
  editingSettings.value = true
}

async function saveSettings() {
  const salary = numOrNull(salaryDraft.value)
  const ratePct = numOrNull(rateDraft.value)
  if (ratePct !== null && (ratePct < 0 || ratePct >= 100)) {
    error.value = '開心Pool 利率必須是 0–100%'
    return
  }
  const bill = numOrNull(billDraft.value)
  if (bill !== null && bill < 0) {
    error.value = '繳費金額不能是負數'
    return
  }
  try {
    await api.updateMonthSettings({
      salary,
      pool_rate: ratePct === null ? null : ratePct / 100,
      pool_rate_year: year.value,
      // Empty keeps the stored value? No — the forecast reads the effective
      // amount; blank resets to the 2158 default.
      bill_amount: bill,
    })
    editingSettings.value = false
    message.value = '已儲存設定'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

const editingAsset = ref<ManualAsset | 'new' | null>(null)
const assetDraft = ref<{
  label: string
  kind: ManualAssetKind
  liquidity: ManualAssetLiquidity
  amount: string | number
}>({
  label: '',
  kind: 'cash',
  liquidity: 'long',
  amount: '',
})

function startAssetEdit(asset: ManualAsset | 'new') {
  editingAsset.value = asset
  assetDraft.value =
    asset === 'new'
      ? { label: '', kind: 'cash', liquidity: 'long', amount: '' }
      : {
          label: asset.label,
          kind: asset.kind,
          liquidity: asset.liquidity,
          amount: asset.amount,
        }
}

async function saveAsset() {
  const amount = numOrNull(assetDraft.value.amount)
  if (assetDraft.value.label.trim() === '' || amount === null) {
    error.value = '名稱及金額必填'
    return
  }
  try {
    const body: NewManualAsset = {
      label: assetDraft.value.label.trim(),
      kind: assetDraft.value.kind,
      liquidity: assetDraft.value.liquidity,
      amount,
    }
    if (editingAsset.value === 'new') {
      await api.createManualAsset(body)
      message.value = '已新增結餘'
    } else if (editingAsset.value) {
      await api.updateManualAsset(editingAsset.value.id, body)
      message.value = '已儲存結餘'
    }
    editingAsset.value = null
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeAsset(asset: ManualAsset) {
  if (!window.confirm(`刪除結餘 ${asset.label}？`)) return
  try {
    await api.deleteManualAsset(asset.id)
    message.value = `已刪除 ${asset.label}`
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <section>
    <div class="card">
      <div class="year-bar">
        <label>
          年份
          <select v-model.number="year" @change="selectYear">
            <option v-for="y in yearOptions" :key="y" :value="y">{{ y }}</option>
          </select>
        </label>
      </div>

      <table v-if="summary" class="year-table">
        <thead>
          <tr>
            <th>年</th>
            <th class="num">總數+</th>
            <th class="num">平均總數</th>
            <th class="num">支出</th>
            <th class="num">平均支出</th>
            <th class="num">生活平均支出</th>
            <th class="num">娛樂支出</th>
            <th class="num">利息回報</th>
            <th class="num">平均回報</th>
            <th class="num">投資純利</th>
            <th class="num">開心Pool結餘</th>
            <th class="num">Irene + 開心Pool</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="y in recentYears"
            :key="y.year"
            class="year-row"
            :class="{ active: y.year === year }"
          >
            <td>{{ y.year }}</td>
            <td class="num">{{ fmtMoney(y.total_change_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.total_change_avg) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.spend_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.spend_avg) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.living_avg) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.entertainment_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.interest_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.interest_avg) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.net_investment) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.pool_balance) || '—' }}</td>
            <td class="num">{{ fmtMoney(y.pool_input_sum) || '—' }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="summary" class="totals-grid" aria-label="歷年平均">
        <div class="total-card">
          <span>平均 Changed</span>
          <strong>{{ fmtMoney(summary.running.total_change_avg) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>平均 流動資產 Changed</span>
          <strong>{{ fmtMoney(summary.running.liquid_change_avg) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>平均 存</span>
          <strong>{{ fmtMoney(summary.running.saved_avg) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>平均 利息</span>
          <strong>{{ fmtMoney(summary.running.interest_avg) || '—' }}</strong>
        </div>
      </div>


      <table>
        <thead>
          <tr>
            <th>月份</th>
            <th class="num">總數</th>
            <th class="num">變動</th>
            <th class="num">流動資產</th>
            <th class="num">流動資產 變動</th>
            <th class="num">月初(出糧後)</th>
            <th class="num">調整</th>
            <th class="num">月尾(出糧前)</th>
            <th class="num">月支出</th>
            <th class="num">生活支出</th>
            <th class="num" title="與去年同月生活支出比較">生活按年</th>
            <th class="num">存</th>
            <th class="num">利息</th>
            <th class="num">娛樂支出</th>
            <th class="num">Irene+開心Pool</th>
            <th>備註</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="m in months"
            :key="m.month"
            :class="{ active: selectedMonth === m.month }"
            class="month-row"
            @click="selectMonth(m)"
          >
            <td>{{ fmtMonth(m.month) }}</td>
            <td
              class="num"
              :class="{ 'live-value': m.total_assets_live }"
              :title="m.total_assets_live ? '即時計算' : ''"
            >
              {{ fmtMoney(m.total_assets) || '—' }}
            </td>
            <td class="num" :class="signClass(m.total_change)">
              {{ fmtMoney(m.total_change) || '—' }}
            </td>
            <td
              class="num"
              :class="{ 'live-value': m.liquid_assets_live }"
              :title="m.liquid_assets_live ? '即時計算' : ''"
            >
              {{ fmtMoney(m.liquid_assets) || '—' }}
            </td>
            <td class="num" :class="signClass(m.liquid_change)">
              {{ fmtMoney(m.liquid_change) || '—' }}
            </td>
            <td class="num">{{ fmtMoney(m.start_cash) || '—' }}</td>
            <td class="num">{{ fmtMoney(m.adjustment_sum) || '—' }}</td>
            <td class="num">
              {{ fmtMoney(m.end_cash) || '—'
              }}<span v-if="m.end_cash_override != null" class="muted" title="手動月尾">*</span>
            </td>
            <td class="num">{{ fmtMoney(m.month_spend) || '—' }}</td>
            <td class="num">{{ fmtMoney(m.living_spend) || '—' }}</td>
            <td class="num" :class="yoyClass(m.living_yoy)">
              {{ fmtPercent(m.living_yoy) || '—' }}
            </td>
            <td class="num" :class="signClass(m.saved)">{{ fmtMoney(m.saved) || '—' }}</td>
            <td class="num">{{ fmtMoney(m.interest) || '—' }}</td>
            <td class="num">{{ fmtMoney(m.entertainment_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(m.pool_input) || '—' }}</td>
            <td class="muted">{{ m.note ?? '' }}</td>
          </tr>
          <tr v-if="months.length === 0">
            <td colspan="16" class="muted">沒有月份 — 執行匯入或新增月份</td>
          </tr>
        </tbody>
      </table>

      <form class="inline-form" @submit.prevent="addMonth">
        <label>
          新增月份
          <input v-model="newMonth" type="month" :placeholder="defaultNewMonth()" />
        </label>
        <div class="form-actions">
          <button type="submit">新增月份</button>
        </div>
      </form>
    </div>

    <div v-if="detail" class="card">
      <h3>{{ fmtMonth(detail.month.month) }}</h3>
      <form class="inline-form" @submit.prevent="saveMonth">
        <label>
          月初
          <input v-model="monthDraft.start_cash" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          薪金
          <input v-model="monthDraft.salary" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          月尾
          <input
            v-model="monthDraft.end_cash_override"
            type="number"
            step="any"
            inputmode="decimal"
            placeholder="自動"
          />
          <small class="muted">留空＝下一月月初 − 薪金</small>
        </label>
        <label>
          利息
          <span>{{ fmtMoney(detail.month.interest) || '0' }}</span>
          <small class="muted">＝自動項目＋下方利息項目合計</small>
        </label>
        <label>
          娛樂支出
          <span>{{ fmtMoney(detail.month.entertainment_sum) || '0' }}</span>
          <small class="muted">＝下方娛樂項目合計</small>
        </label>
        <label>
          Irene + 開心Pool
          <input v-model="monthDraft.pool_input" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          Note
          <input v-model="monthDraft.note" />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <template v-if="selectedIsCurrentOrLater">
            <button type="button" class="link" @click="recapture">
              重新擷取月初/總數/流動
            </button>
            <button type="button" class="link" @click="goLive">改為即時</button>
          </template>
          <button type="button" class="link danger" @click="removeMonth">刪除月份</button>
        </div>
      </form>
      <p class="muted totals-hint">
        總數 {{ fmtMoney(detail.month.total_assets) || '—'
        }}<template v-if="detail.month.total_assets_live">（即時）</template> · 流動資產
        {{ fmtMoney(detail.month.liquid_assets) || '—'
        }}<template v-if="detail.month.liquid_assets_live">（即時）</template>
      </p>

      <div v-for="category in CATEGORY_ORDER" :key="category" class="item-group">
        <h4>{{ CATEGORY_LABELS[category] }}</h4>
        <table>
          <tbody>
            <tr v-for="item in itemsOf(category)" :key="item.id">
              <template v-if="editingItem?.id === item.id">
                <td><input v-model="itemDraft.label" placeholder="標籤" /></td>
                <td class="num">
                  <input
                    v-model="itemDraft.amount"
                    type="number"
                    step="any"
                    inputmode="decimal"
                  />
                </td>
                <td>
                  <input v-model="itemDraft.note" placeholder="備註" />
                  <label v-if="item.category === 'entertainment'" class="muted">
                    <input v-model="itemDraft.exclude_from_living" type="checkbox" />
                    不計入生活支出
                  </label>
                </td>
                <td class="row-actions">
                  <button type="button" class="link" @click="saveItem">儲存</button>
                  <button type="button" class="link" @click="editingItem = null">取消</button>
                </td>
              </template>
              <template v-else>
                <td>
                  {{ item.label ?? '' }}
                  <small v-if="item.note" class="muted note">{{ item.note }}</small>
                  <small v-if="item.exclude_from_living" class="muted note">不計入生活支出</small>
                </td>
                <td class="num" :class="signClass(item.amount)">{{ fmtMoney(item.amount) }}</td>
                <td class="muted">{{ item.auto_key ? '自動' : '' }}</td>
                <td class="row-actions">
                  <button type="button" class="link" @click="startItemEdit(item)">編輯</button>
                  <button type="button" class="link danger" @click="removeItem(item)">刪除</button>
                </td>
              </template>
            </tr>
            <tr v-if="hasImportedTotal && suggestionsOf(category).length" class="hint-row">
              <td colspan="4" class="muted">
                此月已有匯入的調整總額，接受建議前請確認未重複計算。
              </td>
            </tr>
            <tr
              v-for="suggestion in suggestionsOf(category)"
              :key="suggestion.auto_key"
              class="suggestion-row"
            >
              <td>
                {{ suggestion.label ?? suggestion.auto_key }}
                <small class="muted note">{{ suggestion.source }}</small>
              </td>
              <td class="num" :class="signClass(suggestion.amount)">
                {{ fmtMoney(suggestion.amount) }}
              </td>
              <td class="muted">建議</td>
              <td class="row-actions">
                <button type="button" class="link" @click="acceptSuggestion(suggestion)">
                  接受
                </button>
                <button type="button" class="link" @click="dismissSuggestion(suggestion)">
                  忽略
                </button>
              </td>
            </tr>
            <template v-if="category === 'interest'">
              <tr
                v-for="(component, index) in interestAuto"
                :key="`auto-${index}`"
                class="suggestion-row"
                :class="{ 'pending-interest': !component.received }"
              >
                <td>
                  {{ component.label ?? '' }}
                  <small class="muted note">{{
                    INTEREST_SOURCE_LABELS[component.source] ?? component.source
                  }}</small>
                </td>
                <td class="num" :class="signClass(component.amount ?? 0)">
                  {{ fmtMoney(component.amount) || '—' }}
                </td>
                <td class="muted">{{ component.received ? '自動' : '未收' }}</td>
                <td></td>
              </tr>
            </template>
            <tr
              v-if="
                !itemsOf(category).length &&
                !suggestionsOf(category).length &&
                (category !== 'interest' || !interestAuto.length)
              "
            >
              <td colspan="4" class="muted">沒有項目</td>
            </tr>
          </tbody>
        </table>
      </div>

      <form class="inline-form" @submit.prevent="addItem">
        <h4>新增項目</h4>
        <label>
          類別
          <select v-model="newItem.category">
            <option v-for="c in CATEGORY_ORDER" :key="c" :value="c">{{ CATEGORY_LABELS[c] }}</option>
          </select>
        </label>
        <label>
          標籤
          <input v-model="newItem.label" />
        </label>
        <label>
          金額
          <input v-model="newItem.amount" type="number" step="any" inputmode="decimal" required />
        </label>
        <label>
          備註
          <input v-model="newItem.note" />
        </label>
        <label v-if="newItem.category === 'entertainment'">
          <input v-model="newItem.exclude_from_living" type="checkbox" />
          不計入生活支出
        </label>
        <div class="form-actions">
          <button type="submit">新增項目</button>
        </div>
      </form>
    </div>

    <div class="card">
      <h3>
        <button type="button" class="link" @click="settingsOpen = !settingsOpen">
          設定 {{ settingsOpen ? '▾' : '▸' }}
        </button>
      </h3>
      <template v-if="settingsOpen">
        <table>
          <thead>
            <tr>
              <th>名稱</th>
              <th>類別</th>
              <th>流動性</th>
              <th class="num">金額</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="asset in assets" :key="asset.id">
              <td>{{ asset.label }}</td>
              <td>{{ asset.kind === 'cash' ? '活期' : '資產' }}</td>
              <td>
                {{ asset.kind === 'asset' ? (asset.liquidity === 'short' ? '短期' : '長期') : '' }}
              </td>
              <td class="num">{{ fmtMoney(asset.amount) }}</td>
              <td class="row-actions">
                <button type="button" class="link" @click="startAssetEdit(asset)">編輯</button>
                <button type="button" class="link danger" @click="removeAsset(asset)">刪除</button>
              </td>
            </tr>
            <tr v-if="assets.length === 0">
              <td colspan="5" class="muted">沒有手動結餘</td>
            </tr>
          </tbody>
        </table>
        <button type="button" class="link" @click="startAssetEdit('new')">新增結餘</button>

        <form v-if="editingAsset" class="inline-form" @submit.prevent="saveAsset">
          <h4>{{ editingAsset === 'new' ? '新增結餘' : `編輯 ${editingAsset.label}` }}</h4>
          <label>
            名稱
            <input v-model="assetDraft.label" required />
          </label>
          <label>
            類別
            <select v-model="assetDraft.kind">
              <option value="cash">活期</option>
              <option value="asset">資產</option>
            </select>
          </label>
          <label v-if="assetDraft.kind === 'asset'">
            流動性
            <select v-model="assetDraft.liquidity">
              <option value="short">短期</option>
              <option value="long">長期</option>
            </select>
          </label>
          <label>
            金額
            <input v-model="assetDraft.amount" type="number" step="any" inputmode="decimal" required />
          </label>
          <div class="form-actions">
            <button type="submit">儲存</button>
            <button type="button" class="link" @click="editingAsset = null">取消</button>
          </div>
        </form>

        <p class="muted settings-line">
          薪金 {{ fmtMoney(settings?.salary) || '—' }} · 開心Pool {{ year }} 利率
          {{ fmtPercent(year === summary?.pool_rate_year ? summary?.pool_rate : null) || '—' }} ·
          繳費 {{ fmtMoney(settings?.bill_amount) || '—' }}
          <button type="button" class="link" @click="startSettingsEdit">編輯</button>
        </p>
        <form v-if="editingSettings" class="inline-form" @submit.prevent="saveSettings">
          <h4>設定</h4>
          <label>
            薪金
            <input v-model="salaryDraft" type="number" step="any" inputmode="decimal" />
          </label>
          <label>
            開心Pool 利率（{{ year }}，%）
            <input v-model="rateDraft" type="number" step="any" inputmode="decimal" placeholder="33.7" />
          </label>
          <label>
            預測繳費（季繳差餉；留空回復 2158）
            <input v-model="billDraft" type="number" step="any" inputmode="decimal" />
          </label>
          <div class="form-actions">
            <button type="submit">儲存</button>
            <button type="button" class="link" @click="editingSettings = false">取消</button>
          </div>
        </form>
      </template>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.year-table {
  margin: 0.75rem 0;
}
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
  gap: 0.75rem;
  margin-bottom: 0.5rem;
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
.year-row.active td {
  background: var(--surface-alt);
}
.year-bar {
  margin-bottom: 0.25rem;
}
.year-bar label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.85rem;
}
.month-row {
  cursor: pointer;
}
.month-row.active td {
  background: var(--surface-alt);
}
.live-value {
  font-style: italic;
}
.item-group {
  margin-top: 0.75rem;
}
.item-group h4 {
  margin: 0.25rem 0;
}
.note {
  display: block;
  font-size: 0.75rem;
  font-family: monospace;
  white-space: normal;
}
.suggestion-row td {
  border-style: dashed;
  color: var(--muted);
}
/* Pending interest components (deposit not yet 收訖): preview only. */
.pending-interest td {
  opacity: 0.55;
}
.inline-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
}
.inline-form h4 {
  grid-column: 1 / -1;
  margin: 0;
}
.inline-form label {
  display: flex;
  flex-direction: column;
  font-size: 0.8rem;
  gap: 0.2rem;
}
.form-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
.totals-hint {
  margin: 0.5rem 0;
  font-size: 0.8rem;
}
.settings-line {
  font-size: 0.85rem;
}
</style>
