<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  api,
  type MonthDetail,
  type MonthItem,
  type MonthItemCategory,
  type MonthStatPatch,
  type MonthSuggestion,
} from '../api'
import { fmtMoney, fmtMonth, signClass, todayIso } from '../format'
import { numOrNull } from '../parse'
import type { PageRun } from '../page'

const props = defineProps<{ detail: MonthDetail; run: PageRun }>()
const emit = defineEmits<{ deleted: [] }>()

// History rows never derive live (the backend keeps their NULL totals as
// absent), so re-snapshotting or unfreezing only applies from this month on.
const selectedIsCurrentOrLater = computed(
  () => props.detail.month.month.slice(0, 7) >= todayIso().slice(0, 7),
)

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
const interestAuto = computed(() => props.detail.interest_auto)

function itemsOf(category: MonthItemCategory): MonthItem[] {
  return props.detail.items.filter((item) => item.category === category)
}

function suggestionsOf(category: MonthItemCategory): MonthSuggestion[] {
  const all = props.detail.suggestions.filter((s) => s.category === category)
  return [...all.filter((s) => !s.dismissed), ...all.filter((s) => s.dismissed)]
}

/** The imported month's G total already folded events into one adjustment. */
const hasImportedTotal = computed(() =>
  props.detail.items.some((item) => item.auto_key === null && item.note?.startsWith('=')),
)

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
  const month = props.detail.month
  monthDraft.value = {
    start_cash: month.start_cash ?? '',
    salary: month.salary ?? '',
    end_cash_override: month.end_cash_override ?? '',
    pool_input: month.pool_input ?? '',
    note: month.note ?? '',
  }
}

// Reset the draft only when the selected month changes — a reload of the same
// month (after an item save, say) must not clobber in-flight figure edits.
watch(() => props.detail.month.month, startEdit, { immediate: true })

function saveMonth() {
  const patch: MonthStatPatch = {
    start_cash: numOrNull(monthDraft.value.start_cash),
    salary: numOrNull(monthDraft.value.salary),
    end_cash_override: numOrNull(monthDraft.value.end_cash_override),
    pool_input: numOrNull(monthDraft.value.pool_input) ?? 0,
    note: monthDraft.value.note.trim() || null,
  }
  props.run(() => api.patchMonth(props.detail.month.month, patch), {
    message: '已儲存月份',
  })
}

function recapture() {
  props.run(() => api.patchMonth(props.detail.month.month, { recapture: true }), {
    message: '已重新擷取月初及即時總數',
  })
}

function goLive() {
  props.run(
    () =>
      api.patchMonth(props.detail.month.month, {
        total_assets: null,
        liquid_assets: null,
      }),
    { message: '已改為即時計算' },
  )
}

function removeMonth() {
  const month = props.detail.month.month
  props.run(async () => {
    await api.deleteMonth(month)
    emit('deleted')
  }, {
    confirm: `刪除 ${fmtMonth(month)}？其項目會一併刪除。`,
    message: `已刪除 ${fmtMonth(month)}`,
  })
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

function saveItem() {
  const item = editingItem.value
  if (!item) return
  props.run(async () => {
    const amount = numOrNull(itemDraft.value.amount)
    if (amount === null) {
      throw new Error('金額必須是數字')
    }
    await api.updateMonthItem(item.id, {
      label: itemDraft.value.label.trim() || null,
      amount,
      exclude_from_living:
        item.category === 'entertainment' ? itemDraft.value.exclude_from_living : false,
      note: itemDraft.value.note.trim() || null,
    })
    editingItem.value = null
  }, { message: '已儲存項目' })
}

function removeItem(item: MonthItem) {
  props.run(() => api.deleteMonthItem(item.id), {
    confirm: `刪除項目 ${item.label ?? item.amount}？`,
    message: '已刪除項目',
  })
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

function addItem() {
  props.run(async () => {
    const amount = numOrNull(newItem.value.amount)
    if (amount === null) {
      throw new Error('金額必須是數字')
    }
    await api.createMonthItem(props.detail.month.month, {
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
  }, { message: '已新增項目' })
}

function acceptSuggestion(suggestion: MonthSuggestion) {
  props.run(
    () =>
      api.createMonthItem(props.detail.month.month, {
        category: suggestion.category,
        label: suggestion.label,
        amount: suggestion.amount,
        auto_key: suggestion.auto_key,
      }),
    { message: '已接受建議' },
  )
}

function dismissSuggestion(suggestion: MonthSuggestion) {
  props.run(() => api.dismissMonthItem(props.detail.month.month, suggestion.auto_key), {
    message: '已忽略建議',
  })
}

function undismissSuggestion(suggestion: MonthSuggestion) {
  props.run(() => api.undismissMonthItem(props.detail.month.month, suggestion.auto_key), {
    message: '已取消忽略',
  })
}
</script>

<template>
  <div class="card">
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
          <button type="button" class="link" @click="recapture">重新擷取月初/總數/流動</button>
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
                <input v-model="itemDraft.amount" type="number" step="any" inputmode="decimal" />
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
            :class="{ dismissed: suggestion.dismissed }"
          >
            <td>
              {{ suggestion.label ?? suggestion.auto_key }}
              <small class="muted note">{{ suggestion.source }}</small>
            </td>
            <td class="num" :class="signClass(suggestion.amount)">
              {{ fmtMoney(suggestion.amount) }}
            </td>
            <td class="muted">{{ suggestion.dismissed ? '忽略（不計）' : '建議' }}</td>
            <td class="row-actions">
              <template v-if="detail.suggestions_actionable">
                <button type="button" class="link" @click="acceptSuggestion(suggestion)">
                  接受
                </button>
                <button
                  v-if="suggestion.dismissed"
                  type="button"
                  class="link"
                  @click="undismissSuggestion(suggestion)"
                >
                  取消忽略
                </button>
                <button
                  v-else
                  type="button"
                  class="link"
                  @click="dismissSuggestion(suggestion)"
                >
                  忽略
                </button>
              </template>
            </td>
          </tr>
          <template v-if="category === 'interest'">
            <tr
              v-for="(component, index) in interestAuto"
              :key="`auto-${index}`"
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
</template>

<style scoped>
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
.suggestion-row td,
.pending-interest td {
  border-style: dashed;
  color: var(--muted);
}
.suggestion-row.dismissed td {
  opacity: 0.55;
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
</style>
