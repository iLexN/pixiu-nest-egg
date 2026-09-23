<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  api,
  ApiError,
  type YearReviewPatch,
  type YearReviewResponse,
  type YearReviewRow,
} from '../api'
import { fmtMoney, fmtPercent, signClass } from '../format'

const response = ref<YearReviewResponse | null>(null)
const error = ref('')
const years = computed(() => response.value?.years ?? [])

async function load() {
  error.value = ''
  try {
    response.value = await api.yearReview()
  } catch (err) {
    response.value = null
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)

type EditField = keyof YearReviewPatch

interface MetricRow {
  label: string
  get: (row: YearReviewRow) => number | null
  /** 'pct' renders as a percentage; default money. */
  fmt?: 'pct'
  /** YoY ratio shown inline next to the value. */
  yoy?: (row: YearReviewRow) => number | null
  /** Editable manual field, if this row takes input. */
  field?: EditField
}

const sections: { label: string; rows: MetricRow[] }[] = [
  {
    label: '帳目',
    rows: [
      { label: '總數+', get: (r) => r.ledger.asset_gain, yoy: (r) => r.ledger.asset_gain_yoy },
      { label: '平均總數+', get: (r) => r.ledger.asset_gain_avg },
      { label: '支出', get: (r) => r.ledger.spend, yoy: (r) => r.ledger.spend_yoy },
      { label: '平均支出', get: (r) => r.ledger.spend_avg },
      {
        label: '生活平均支出',
        get: (r) => r.ledger.living_avg,
        yoy: (r) => r.ledger.living_yoy,
      },
      {
        label: '開心 Pool 收入',
        get: (r) => r.ledger.pool_income,
        yoy: (r) => r.ledger.pool_income_yoy,
      },
      { label: '開心 Pool 支出', get: (r) => r.ledger.pool_spend },
      { label: '開心 Pool 結餘', get: (r) => r.ledger.pool_balance },
    ],
  },
  {
    label: '投資',
    rows: [
      { label: '利息回報', get: (r) => r.investment.interest },
      {
        label: '平均回報',
        get: (r) => r.investment.interest_avg,
        yoy: (r) => r.investment.interest_avg_yoy,
      },
      { label: '投資P/L', get: (r) => r.investment.sold_pl, field: 'sold_pl' },
      { label: '投資純利', get: (r) => r.investment.net_investment },
      { label: 'IBKR 轉入', get: (r) => r.investment.transferred },
      {
        label: 'invested 調整',
        get: (r) => r.record?.invested_adjustment ?? null,
        field: 'invested_adjustment',
      },
      {
        label: 'invested',
        get: (r) => r.investment.invested,
        yoy: (r) => r.investment.invested_yoy,
      },
      { label: 'invested %', get: (r) => r.investment.invested_pct, fmt: 'pct' },
      { label: 'Irene + 開心 Pool', get: (r) => r.investment.irene_pool },
    ],
  },
  {
    label: '資產',
    rows: [
      { label: '債券 本金', get: (r) => r.assets.bond_principal, field: 'bond_principal' },
      { label: '債券 利息', get: (r) => r.assets.bond_interest, field: 'bond_interest' },
      { label: '債券 率', get: (r) => r.assets.bond_rate, fmt: 'pct' },
      { label: '股票 成本', get: (r) => r.assets.stock_cost },
      { label: '股票 派息', get: (r) => r.assets.stock_dividends },
      { label: '股票 率', get: (r) => r.assets.stock_rate, fmt: 'pct' },
      { label: '股票 市值', get: (r) => r.assets.stock_now_value },
      { label: '股票 市值率', get: (r) => r.assets.stock_value_rate, fmt: 'pct' },
      { label: '定期 本金', get: (r) => r.assets.deposit_principal, field: 'deposit_principal' },
      { label: '定期 利息', get: (r) => r.assets.deposit_interest, field: 'deposit_interest' },
    ],
  },
  {
    label: '收入/存',
    rows: [
      { label: '回報率 收入÷成本', get: (r) => r.assets.income_cost_rate, fmt: 'pct' },
      { label: '回報率 總回報÷市值', get: (r) => r.assets.total_value_rate, fmt: 'pct' },
      { label: '回報率 收入÷市值', get: (r) => r.assets.income_value_rate, fmt: 'pct' },
      {
        label: '收入',
        get: (r) => r.assets.income,
        field: 'income',
        yoy: (r) => r.assets.income_yoy,
      },
      { label: '平均收入', get: (r) => r.assets.income_avg },
      { label: '存', get: (r) => r.assets.saved },
      { label: '平均存', get: (r) => r.assets.saved_avg },
      { label: '存 %', get: (r) => r.assets.saved_pct, fmt: 'pct' },
    ],
  },
]

/** The stored value for an editable field, if the record carries one. */
function stored(row: YearReviewRow, field: EditField): number | null {
  if (field === 'sold_pl') return row.investment.sold_pl
  return row.record?.[field as Exclude<EditField, 'sold_pl'>] ?? null
}

/** Whether the figure on screen is a stored manual/override value. */
function manual(row: YearReviewRow, field: EditField): boolean {
  return stored(row, field) !== null
}

function display(value: number | null, fmt?: 'pct'): string {
  if (value === null) return '—'
  return fmt === 'pct' ? fmtPercent(value) : fmtMoney(value)
}

const editingCell = ref<string | null>(null)
const draft = ref('')

function startEdit(row: YearReviewRow, field: EditField, current: number | null) {
  editingCell.value = `${row.year}:${field}`
  draft.value = current === null ? '' : String(current)
}

async function saveEdit(row: YearReviewRow, field: EditField) {
  const text = String(draft.value).trim()
  const parsed = text === '' ? null : Number(text)
  if (parsed !== null && !Number.isFinite(parsed)) {
    error.value = '數值必須是數字'
    return
  }
  // sold_pl and the invested adjustment may legitimately be negative.
  if (parsed !== null && parsed < 0 && field !== 'sold_pl' && field !== 'invested_adjustment') {
    error.value = '數值必須是非負數字'
    return
  }
  try {
    await api.updateYearReview(row.year, { [field]: parsed })
    editingCell.value = null
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}
</script>

<template>
  <section class="card">
    <h3>回顧</h3>
    <p v-if="error" class="error">{{ error }}</p>
    <p class="muted">
      收入、invested 調整、投資P/L 與歷史債券/定期為手動或匯入凍結值（*），點擊可修改；其餘即時計算。
    </p>

    <table class="review">
      <thead>
        <tr>
          <th></th>
          <th v-for="year in years" :key="year.year" class="num">{{ year.year }}</th>
        </tr>
      </thead>
      <tbody v-for="section in sections" :key="section.label">
        <tr class="section">
          <th :colspan="years.length + 1">{{ section.label }}</th>
        </tr>
        <tr v-for="metric in section.rows" :key="metric.label">
          <td>{{ metric.label }}</td>
          <td
            v-for="year in years"
            :key="year.year"
            class="num"
            :class="{ 'edit-cell': metric.field }"
          >
            <template v-if="metric.field">
              <template v-if="editingCell === `${year.year}:${metric.field}`">
                <input
                  v-model="draft"
                  type="number"
                  step="any"
                  @keyup.enter="saveEdit(year, metric.field)"
                />
                <button type="button" class="link" @click="saveEdit(year, metric.field)">
                  儲存
                </button>
                <button type="button" class="link" @click="editingCell = null">取消</button>
              </template>
              <button
                v-else
                type="button"
                class="link"
                :class="{ manual: manual(year, metric.field) }"
                :title="manual(year, metric.field) ? '手動/凍結值 — 清空以即時計算' : ''"
                @click="startEdit(year, metric.field, metric.get(year))"
              >
                {{ display(metric.get(year)) === '—' ? '輸入' : display(metric.get(year))
                }}{{ manual(year, metric.field) ? '*' : '' }}
              </button>
            </template>
            <template v-else>{{ display(metric.get(year), metric.fmt) }}</template>
            <span
              v-if="metric.yoy && metric.yoy(year) !== null"
              class="yoy"
              :class="signClass(metric.yoy(year))"
              >{{ fmtPercent(metric.yoy(year)) }}</span
            >
          </td>
        </tr>
      </tbody>
    </table>
  </section>

  <section class="card guide">
    <h3>年末要做的事</h3>
    <ol>
      <li>
        <b>凍結年末數字</b> — 到 股票 → 總覽 → 每年總覽，在該年那列按「凍結」（港股、美股各一次）。
        這會把年末成本與總市值存起來；不凍結的話，往年市值之後會變空白。
      </li>
      <li>
        <b>輸入賣出損益</b> — 在每年總覽的「賣出損益」欄或本頁的「投資P/L」輸入該年已實現損益；
        沒有賣出就輸入 0（投資純利要有了這個數才會顯示）。
      </li>
      <li>
        <b>輸入收入</b> — 在本頁填該年的收入；invested 已自動計入港股買賣淨額和
        當年 IBKR 轉入，還要包含其他的錢（例如買債券）再填「invested 調整」。
      </li>
    </ol>
    <p class="muted">
      年初不用做什麼：新一年的列會自動出現；開心Pool 利率沒變就不用設，薪金變了才去
      月結設定更新。債券/定期不用手動填——已到期的記錄仍會留著，數字會繼續自己算。
    </p>
  </section>
</template>

<style scoped>
.review {
  width: auto;
  font-size: 0.84rem;
}
.review th,
.review td {
  padding: 0.4rem 0.7rem;
}
.review td.num,
.review th.num {
  min-width: 8.5rem;
}
.review .section th {
  text-align: left;
  background: var(--surface-alt);
  border-bottom: 1px solid var(--border);
}
.yoy {
  display: block;
  font-size: 0.78em;
  opacity: 0.85;
}
.edit-cell input {
  width: 6.5rem;
}
.edit-cell .manual {
  text-decoration: underline dotted;
}
.guide {
  margin-top: 1.25rem;
}
.guide ol {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}
.guide li {
  margin-bottom: 0.4rem;
}
</style>
