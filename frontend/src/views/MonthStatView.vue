<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  api,
  type AppSettings,
  type CashAccount,
  type ManualAsset,
  type MonthDetail,
  type MonthStat,
  type MonthSummary,
} from '../api'
import MonthDetailPanel from '../components/MonthDetailPanel.vue'
import MonthSettingsPanel from '../components/MonthSettingsPanel.vue'
import { fmtMoney, fmtMonth, fmtPercent, signClass, todayIso } from '../format'
import { usePage } from '../page'

const summary = ref<MonthSummary | null>(null)
const months = ref<MonthStat[]>([])
const detail = ref<MonthDetail | null>(null)
const settings = ref<AppSettings | null>(null)
const assets = ref<ManualAsset[]>([])
const accounts = ref<CashAccount[]>([])

const currentYear = new Date().getFullYear()
const year = ref(currentYear)
const selectedMonth = ref<string | null>(null)

const { message, error, reload, run } = usePage(load)

const yearOptions = computed(() => {
  const set = new Set<number>([currentYear])
  for (const y of summary.value?.years ?? []) set.add(y.year)
  return [...set].sort((a, b) => a - b)
})

/** The sheet's A1:N4 yearly block — last 3 years. */
const recentYears = computed(() => (summary.value?.years ?? []).slice(-3))

/** 生活按年: spending less than last year is green, >2% inflation is red. */
function yoyClass(value: number | null): string {
  if (value === null) return ''
  if (value < 0) return 'positive'
  if (value > 0.02) return 'negative'
  return ''
}

async function load() {
  const [summaryData, monthsData, allMonths, settingsData, assetsData, accountsData] =
    await Promise.all([
      api.monthSummary(),
      api.listMonths(year.value),
      api.listMonths(),
      api.getSettings(),
      api.listManualAssets(),
      api.listCashAccounts(),
    ])
  summary.value = summaryData
  months.value = monthsData
  latestStoredMonth.value = allMonths.at(-1)?.month ?? null
  settings.value = settingsData
  assets.value = assetsData
  accounts.value = accountsData
  if (!newMonth.value) newMonth.value = defaultNewMonth()
  if (selectedMonth.value) {
    detail.value = await api.monthDetail(selectedMonth.value)
  }
}

function selectYear() {
  run(async () => {
    months.value = await api.listMonths(year.value)
    if (selectedMonth.value && !selectedMonth.value.startsWith(`${year.value}-`)) {
      selectedMonth.value = null
      detail.value = null
    }
  }, { reload: false })
}

function selectMonth(month: MonthStat) {
  selectedMonth.value = month.month
  run(async () => {
    detail.value = await api.monthDetail(month.month)
  }, { reload: false })
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

function addMonth() {
  const ym = newMonth.value || defaultNewMonth()
  run(async () => {
    // New months start live (即時); 重新擷取 freezes them at month close.
    await api.patchMonth(ym, { total_assets: null, liquid_assets: null })
    newMonth.value = ''
    await reload()
    const created = months.value.find((m) => m.month === `${ym}-01`)
    if (created) {
      selectedMonth.value = created.month
      detail.value = await api.monthDetail(created.month)
    } else {
      selectedMonth.value = `${ym}-01`
      detail.value = await api.monthDetail(selectedMonth.value)
    }
  }, {
    message: `已新增 ${ym}`,
    reload: false,
  })
}

function onPanelDeleted() {
  selectedMonth.value = null
  detail.value = null
}

onMounted(reload)
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

    <MonthDetailPanel
      v-if="detail"
      :detail="detail"
      :run="run"
      @deleted="onPanelDeleted"
    />

    <MonthSettingsPanel
      :settings="settings"
      :summary="summary"
      :accounts="accounts"
      :assets="assets"
      :year="year"
      :run="run"
    />

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error?.message }}</p>
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
.inline-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
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
</style>
