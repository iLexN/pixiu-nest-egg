<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import {
  api,
  errorMessage,
  type ForecastCell,
  type ForecastItem,
  type ForecastItemKind,
  type ForecastMonth,
  type ForecastResponse,
  type InterestComponent,
} from '../api'
import DepositForm from './DepositForm.vue'
import { fmtMoney, signClass } from '../format'
import { numOrNull } from '../parse'

const props = defineProps<{ forecast: ForecastResponse }>()

const emit = defineEmits<{ saved: [message: string]; failed: [message: string] }>()

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

/** The month's backend-built cell for a plan row: its plan items, read-only
 * placement/trade/IBKR lines, and the figure the grid shows. */
function cellOf(month: ForecastMonth, kind: ForecastItemKind): ForecastCell | undefined {
  return month.cells.find((cell) => cell.kind === kind)
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
  props.forecast.months.find((month) => month.month === selected.value?.month),
)
const selectedCell = computed(() =>
  selectedMonth.value && selected.value
    ? cellOf(selectedMonth.value, selected.value.kind)
    : undefined,
)
const selectedItems = computed(() => selectedCell.value?.plan_items ?? [])
const selectedPlacements = computed(() => selectedCell.value?.placements ?? [])
const selectedTrades = computed(() => selectedCell.value?.trade_lines ?? [])
const selectedIbkrLines = computed(() => selectedCell.value?.ibkr_lines ?? [])

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

// The parent refetches `forecast` after every save; a fresh object lands
// here and the open cell's drafts rebuild, like load() used to do inline.
watch(
  () => props.forecast,
  () => {
    if (selected.value) syncDrafts()
  },
)

async function saveItem(item: ForecastItem) {
  const draft = drafts.value[item.id]
  if (!draft) return
  const amount = numOrNull(draft.amount)
  if (amount === null) {
    emit('failed', '金額必須是數字')
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
    emit('saved', '已更新預測')
  } catch (err) {
    emit('failed', errorMessage(err))
  }
}

async function addItem() {
  const current = selected.value
  if (!current) return
  const amount = numOrNull(newItem.amount)
  if (amount === null) {
    emit('failed', '金額必須是數字')
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
    emit('saved', '已新增預測項目')
  } catch (err) {
    emit('failed', errorMessage(err))
  }
}

async function removeItem(item: ForecastItem) {
  if (!window.confirm(`刪除預測項目（${fmtMoney(item.amount)}）？`)) return
  try {
    await api.deleteForecastItem(item.id)
    emit('saved', '已刪除預測項目')
  } catch (err) {
    emit('failed', errorMessage(err))
  }
}

/** 複製: duplicate the row's current inputs into the same month next year.
 * The target sits outside the 7-month window — stored but invisible until
 * the months slide forward, so the message names where it landed. */
async function copyItem(item: ForecastItem) {
  const draft = drafts.value[item.id]
  if (!draft) return
  const amount = numOrNull(draft.amount)
  if (amount === null) {
    emit('failed', '金額必須是數字')
    return
  }
  const target = `${Number(item.month.slice(0, 4)) + 1}-${item.month.slice(5, 7)}`
  try {
    await api.createForecastItem(target, {
      kind: item.kind,
      amount,
      note: draft.note.trim() || undefined,
    })
    emit('saved', `已複製到 ${target}`)
  } catch (err) {
    emit('failed', errorMessage(err))
  }
}

function startConvert(item: ForecastItem) {
  converting.value = item
}

function onConverted() {
  converting.value = null
  emit('saved', '已轉為定期')
}
</script>

<template>
  <div class="card forecast">
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
            <td
              v-for="month in forecast.months"
              :key="month.month"
              class="num"
              :class="{ negative: month.cash !== null && month.cash < 20000 }"
            >
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
                selected: selected?.month === month.month && selected?.kind === row.kind,
              }"
              @click="selectCell(month.month, row.kind)"
            >
              {{ fmtMoney(cellOf(month, row.kind)?.figure) || '—' }}
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
                selected: selected?.month === month.month && selected?.kind === 'other',
              }"
              @click="selectCell(month.month, 'other')"
            >
              {{ fmtMoney(cellOf(month, 'other')?.figure) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="selected && selectedMonth && selectedKind" class="plan-editor">
      <h4>{{ selected.month.slice(0, 7) }} · {{ selectedKind.label }}</h4>
      <template v-if="selected.kind === 'interest'">
        <p class="muted">
          總額已含自動利息（到期定期利息／派息／債息）— 此處只編輯額外手動項目
        </p>
        <ul class="component-list">
          <li v-if="!selectedMonth.interest_components.length" class="muted">
            本月沒有自動利息
          </li>
          <li v-for="(component, index) in selectedMonth.interest_components" :key="index">
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
      <!-- Real deposits already placed in the month — read-only, above the
           editable plan items. -->
      <div v-for="p in selectedPlacements" :key="`dep-${p.deposit_id}`" class="plan-row muted">
        <span>{{ p.label || `定期 #${p.deposit_id}` }}</span>
        <span class="num">{{ fmtMoney(-p.principal) }}</span>
        <span>{{ p.start_date }}</span>
        <span>已定期</span>
      </div>
      <!-- Real HK trades dated in the month — read-only, above the editable
           plan items. -->
      <div v-for="t in selectedTrades" :key="`trade-${t.trade_id}`" class="plan-row muted">
        <span>{{ t.code }} {{ t.trade_type }}</span>
        <span class="num">{{ fmtMoney(t.amount) }}</span>
        <span>{{ t.trade_date }}</span>
        <span>已交易</span>
      </div>
      <!-- IBKR transfers dated in the month — read-only 已轉帳 lines
           beside the trades. -->
      <div v-for="t in selectedIbkrLines" :key="`ibkr-${t.transfer_id}`" class="plan-row muted">
        <span>IBKR 轉入</span>
        <span class="num">{{ fmtMoney(t.amount) }}</span>
        <span>{{ t.transfer_date }}</span>
        <span>已轉帳</span>
      </div>
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
          <input v-model="drafts[item.id].note" type="text" class="note-input" placeholder="note" />
          <button type="button" class="link" @click="saveItem(item)">儲存</button>
          <button v-if="selectedKind.deposit" type="button" class="link" @click="startConvert(item)">
            轉為定期
          </button>
          <button type="button" class="link" @click="removeItem(item)">刪除</button>
          <button
            v-if="selected.kind === 'tax'"
            type="button"
            class="link"
            title="複製到明年同月"
            @click="copyItem(item)"
          >
            複製
          </button>
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
        <input v-model="newItem.note" type="text" class="note-input" placeholder="note" />
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
</template>

<style scoped>
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
.cell-input {
  width: 8rem;
  text-align: right;
}
.deposit-form {
  margin-top: 0.75rem;
}
</style>
