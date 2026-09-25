<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import DateInput from '../components/DateInput.vue'
import RowActions from '../components/RowActions.vue'
import {
  api,
  ApiError,
  type FamilyDeposit,
  type FamilyDepositSummary,
  type FamilyHolderSummary,
} from '../api'
import { fmtBank, fmtMoney, todayIso } from '../format'

const summary = ref<FamilyDepositSummary | null>(null)
const message = ref('')
const error = ref('')

const DEFAULT_HOLDERS = ['媽媽', '爸爸', 'Irene']
const holderOptions = computed(() => {
  const options = [...DEFAULT_HOLDERS]
  for (const h of summary.value?.holders ?? []) {
    if (!options.includes(h.holder)) options.push(h.holder)
  }
  return options
})

const selectedHolder = ref<string | null>(null)
const visibleHolders = computed<FamilyHolderSummary[]>(() => {
  const holders = summary.value?.holders ?? []
  return selectedHolder.value === null
    ? holders
    : holders.filter((h) => h.holder === selectedHolder.value)
})

const upcoming = computed<FamilyDeposit[]>(() =>
  visibleHolders.value
    .flatMap((h) => h.upcoming)
    .sort((a, b) => a.end_date.localeCompare(b.end_date) || a.sort_order - b.sort_order),
)

async function load() {
  error.value = ''
  try {
    summary.value = await api.familyDepositSummary()
    if (
      selectedHolder.value !== null &&
      !summary.value.holders.some((h) => h.holder === selectedHolder.value)
    ) {
      selectedHolder.value = null
    }
    await loadHistory()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function overdue(deposit: FamilyDeposit): boolean {
  return deposit.received_at === null && deposit.end_date <= (summary.value?.today ?? '')
}

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

// ----- holder note -----

const editingNote = ref<string | null>(null)
const noteDraft = ref('')
const shownNotes = ref(new Set<string>())

function toggleNote(holder: string) {
  const next = new Set(shownNotes.value)
  if (next.has(holder)) {
    next.delete(holder)
  } else {
    next.add(holder)
  }
  shownNotes.value = next
}

function startNote(holder: FamilyHolderSummary) {
  shownNotes.value = new Set(shownNotes.value).add(holder.holder)
  editingNote.value = holder.holder
  noteDraft.value = holder.note ?? ''
}

async function saveNote(holder: string) {
  try {
    await api.updateFamilyHolderNote(holder, noteDraft.value.trim() || null)
    editingNote.value = null
    message.value = '已儲存備註'
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- create / edit form -----

interface DepositDraft {
  holderChoice: string
  holderOther: string
  label: string
  bank: string
  principal: string | number
  interest: string | number
  start_date: string
  end_date: string
  note: string
}

function emptyDraft(): DepositDraft {
  return {
    holderChoice: selectedHolder.value ?? DEFAULT_HOLDERS[0],
    holderOther: '',
    label: '',
    bank: '',
    principal: '',
    interest: '',
    start_date: todayIso(),
    end_date: '',
    note: '',
  }
}

const showForm = ref(false)
const editing = ref<FamilyDeposit | null>(null)
const draft = ref<DepositDraft>(emptyDraft())

function toggleForm() {
  // While editing, 新增定期 switches to a fresh create form instead of closing.
  showForm.value = editing.value ? true : !showForm.value
  editing.value = null
  draft.value = emptyDraft()
}

function cancelForm() {
  editing.value = null
  showForm.value = false
}

function startEdit(deposit: FamilyDeposit) {
  editing.value = deposit
  showForm.value = true
  draft.value = {
    holderChoice: holderOptions.value.includes(deposit.holder) ? deposit.holder : '__other__',
    holderOther: holderOptions.value.includes(deposit.holder) ? '' : deposit.holder,
    label: deposit.label ?? '',
    bank: deposit.bank ?? '',
    principal: deposit.principal === null ? '' : String(deposit.principal),
    interest: deposit.interest === null ? '' : String(deposit.interest),
    start_date: deposit.start_date ?? '',
    end_date: deposit.end_date,
    note: deposit.note ?? '',
  }
}

async function saveDeposit() {
  const holder =
    draft.value.holderChoice === '__other__' ? draft.value.holderOther.trim() : draft.value.holderChoice
  if (!holder) {
    error.value = '持有人必填'
    return
  }
  const body = {
    holder,
    label: draft.value.label.trim() || null,
    bank: draft.value.bank.trim() || null,
    principal: numOrNull(draft.value.principal),
    interest: numOrNull(draft.value.interest),
    start_date: draft.value.start_date || null,
    end_date: draft.value.end_date,
    note: draft.value.note.trim() || null,
  }
  try {
    if (editing.value) {
      await api.updateFamilyDeposit(editing.value.id, body)
      message.value = '已儲存定期'
    } else {
      await api.createFamilyDeposit(body)
      message.value = '已新增定期'
    }
    editing.value = null
    showForm.value = false
    draft.value = emptyDraft()
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function remove(deposit: FamilyDeposit) {
  const name = deposit.label ?? `#${deposit.id}`
  if (!window.confirm(`刪除 ${deposit.holder} 的定期 ${name}（${deposit.end_date} 到期）？`)) return
  try {
    await api.deleteFamilyDeposit(deposit.id)
    message.value = `已刪除定期 ${name}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- 收訖 / 取消收訖 (record only — no bank-in, no month item) -----

const receiving = ref<{
  deposit: FamilyDeposit
  received_at: string
  interest: string | number
} | null>(null)

function startReceive(deposit: FamilyDeposit) {
  receiving.value = {
    deposit,
    received_at: todayIso(),
    interest: deposit.interest ?? '',
  }
}

async function saveReceive() {
  const current = receiving.value
  if (!current) return
  const interest = numOrNull(current.interest)
  if (current.interest !== '' && interest === null) {
    error.value = '實收利息必須是數字'
    return
  }
  try {
    await api.receiveFamilyDeposit(current.deposit.id, {
      received_at: current.received_at || undefined,
      interest: interest ?? undefined,
    })
    receiving.value = null
    message.value = '已記錄收訖'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function unreceive(deposit: FamilyDeposit) {
  const name = deposit.label ?? `#${deposit.id}`
  if (!window.confirm(`取消收訖 ${name}？`)) return
  try {
    await api.unreceiveFamilyDeposit(deposit.id)
    message.value = `已取消收訖 ${name}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- history -----

const history = ref<FamilyDeposit[]>([])
const selectedYear = ref<number>(new Date().getFullYear())

async function loadHistory() {
  editing.value = null
  const years = summary.value?.history_years ?? []
  if (!years.includes(selectedYear.value)) {
    selectedYear.value = years.at(-1) ?? selectedYear.value
  }
  try {
    history.value = await api.listFamilyDeposits({ year: selectedYear.value, order: 'desc' })
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <section>
    <nav v-if="summary && summary.holders.length > 1" class="holder-chips">
      <button :class="{ active: selectedHolder === null }" @click="selectedHolder = null">
        全部
      </button>
      <button
        v-for="h in summary.holders"
        :key="h.holder"
        :class="{ active: selectedHolder === h.holder }"
        @click="selectedHolder = h.holder"
      >
        {{ h.holder }}
      </button>
    </nav>

    <button type="button" class="toggle-form" @click="toggleForm">新增定期</button>
    <form v-if="showForm" class="card deposit-form" @submit.prevent="saveDeposit">
      <h3>
        {{ editing ? `編輯定期 ${editing.label ?? `#${editing.id}`}` : '新增定期' }}
      </h3>
      <div class="grid">
        <label>
          持有人
          <select v-model="draft.holderChoice" required>
            <option v-for="h in holderOptions" :key="h" :value="h">{{ h }}</option>
            <option value="__other__">其他…</option>
          </select>
          <input
            v-if="draft.holderChoice === '__other__'"
            v-model="draft.holderOther"
            type="text"
            placeholder="姓名"
            required
          />
        </label>
        <label>
          id
          <input v-model="draft.label" type="text" placeholder="SC-9179" />
        </label>
        <label>
          銀行
          <input v-model="draft.bank" type="text" list="family-bank-codes" placeholder="SC" />
          <datalist id="family-bank-codes">
            <option value="SC">渣打</option>
            <option value="HS">恒生</option>
          </datalist>
        </label>
        <label>
          input（本金）
          <input v-model="draft.principal" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          利息
          <input v-model="draft.interest" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          開始日
          <DateInput v-model="draft.start_date" />
        </label>
        <label>
          end date
          <DateInput v-model="draft.end_date" required />
        </label>
        <label class="wide">
          note
          <textarea v-model="draft.note" rows="2" placeholder="分段利率等備註"></textarea>
        </label>
      </div>
      <div class="actions">
        <button type="submit">{{ editing ? '儲存' : '新增' }}</button>
        <button type="button" class="secondary" @click="cancelForm">取消</button>
      </div>
    </form>

    <div class="holder-grid">
      <div v-for="holder in visibleHolders" :key="holder.holder" class="card">
        <div class="holder-header">
          <h3>{{ holder.holder }}</h3>
          <div class="total-card">
            <span>活躍本金</span>
            <strong>{{ fmtMoney(holder.active_principal) }}</strong>
          </div>
        </div>

        <div class="note-block">
          <button type="button" class="link" @click="toggleNote(holder.holder)">
            {{
              shownNotes.has(holder.holder) ? '隱藏備註' : holder.note ? '顯示備註' : '備註'
            }}
          </button>
          <template v-if="shownNotes.has(holder.holder)">
            <template v-if="editingNote === holder.holder">
              <textarea v-model="noteDraft" rows="3"></textarea>
              <div>
                <button type="button" @click="saveNote(holder.holder)">儲存</button>
                <button type="button" class="link" @click="editingNote = null">取消</button>
              </div>
            </template>
            <template v-else>
              <p v-if="holder.note" class="note-text">{{ holder.note }}</p>
              <button type="button" class="link" @click="startNote(holder)">
                {{ holder.note ? '編輯備註' : '新增備註' }}
              </button>
            </template>
          </template>
        </div>

      </div>
    </div>

    <div v-if="summary" class="card">
      <h3>未到期定期</h3>
      <table>
        <thead>
          <tr>
            <th>持有人</th>
            <th>end date</th>
            <th>id</th>
            <th>銀行</th>
            <th class="num">input</th>
            <th class="num">利息</th>
            <th class="num">total</th>
            <th>開始日</th>
            <th>note</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="deposit in upcoming" :key="deposit.id">
            <td>{{ deposit.holder }}</td>
            <td>
              {{ deposit.end_date }}
              <small v-if="overdue(deposit)" class="overdue">已到期未收</small>
            </td>
            <td>{{ deposit.label ?? '' }}</td>
            <td>{{ fmtBank(deposit.bank) }}</td>
            <td class="num">{{ fmtMoney(deposit.principal) }}</td>
            <td class="num">{{ fmtMoney(deposit.interest) }}</td>
            <td class="num">{{ fmtMoney(deposit.total) }}</td>
            <td>{{ deposit.start_date ?? '' }}</td>
            <td class="note">{{ deposit.note ?? '' }}</td>
            <td class="row-actions">
              <button type="button" class="link" @click="startReceive(deposit)">收訖</button>
              <RowActions @edit="startEdit(deposit)" @remove="remove(deposit)" />
            </td>
          </tr>
          <tr v-if="upcoming.length === 0">
            <td colspan="10" class="muted">沒有未到期的定期</td>
          </tr>
        </tbody>
      </table>

      <form v-if="receiving" class="inline-form" @submit.prevent="saveReceive">
        <h4>
          收訖 — {{ receiving.deposit.holder }}
          {{ receiving.deposit.label ?? `#${receiving.deposit.id}` }}（{{
            receiving.deposit.end_date
          }}
          到期）
        </h4>
        <label>
          收訖日
          <DateInput v-model="receiving.received_at" required />
        </label>
        <label>
          實收利息
          <input v-model="receiving.interest" type="number" step="any" inputmode="decimal" />
        </label>
        <div class="form-actions">
          <button type="submit">收訖</button>
          <button type="button" class="link" @click="receiving = null">取消</button>
        </div>
      </form>
    </div>

    <div v-if="summary" class="card">
      <div class="history-header">
        <h3>定期記錄</h3>
        <label>
          年份
          <select v-model="selectedYear" @change="loadHistory">
            <option v-for="year in summary.history_years" :key="year" :value="year">
              {{ year }}
            </option>
          </select>
        </label>
      </div>
      <table>
        <thead>
          <tr>
            <th>持有人</th>
            <th>end date</th>
            <th>id</th>
            <th>銀行</th>
            <th class="num">input</th>
            <th class="num">利息</th>
            <th class="num">total</th>
            <th>狀態</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="deposit in history" :key="deposit.id">
            <td>{{ deposit.holder }}</td>
            <td>
              {{ deposit.end_date }}
              <small v-if="overdue(deposit)" class="overdue">已到期未收</small>
            </td>
            <td>{{ deposit.label ?? '' }}</td>
            <td>{{ fmtBank(deposit.bank) }}</td>
            <td class="num">{{ fmtMoney(deposit.principal) }}</td>
            <td class="num">{{ fmtMoney(deposit.interest) }}</td>
            <td class="num">{{ fmtMoney(deposit.total) }}</td>
            <td>{{ deposit.received_at ? `收訖 ${deposit.received_at}` : '未收' }}</td>
            <td class="row-actions">
              <button
                v-if="deposit.received_at"
                type="button"
                class="link"
                @click="unreceive(deposit)"
              >
                取消收訖
              </button>
              <RowActions @edit="startEdit(deposit)" @remove="remove(deposit)" />
            </td>
          </tr>
          <tr v-if="history.length === 0">
            <td colspan="9" class="muted">這年沒有到期記錄</td>
          </tr>
        </tbody>
      </table>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.holder-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  margin-bottom: 1rem;
}
.holder-chips button.active {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.toggle-form {
  margin-bottom: 1rem;
}
.deposit-form h3 {
  margin: 0 0 0.75rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
}
.grid label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.25rem;
}
.wide {
  grid-column: 1 / -1;
}
.actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
.holder-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}
.holder-grid > .card {
  flex: 1 1 260px;
  min-width: 0;
}
.holder-header {
  display: flex;
  align-items: center;
  gap: 1rem;
}
.holder-header h3 {
  margin: 0;
}
.total-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.5rem 0.75rem;
}
.total-card span {
  color: var(--muted);
  display: block;
  font-size: 0.78rem;
}
.total-card strong {
  font-size: 1.05rem;
  font-weight: 650;
}
.note-block {
  margin: 0.5rem 0 0;
}
.note-text {
  white-space: pre-wrap;
  margin: 0 0 0.25rem;
}
.note-block textarea {
  width: 100%;
}
.note-block .link {
  padding: 0;
}
h4 {
  margin: 1.5rem 0 0.5rem;
}
.note {
  max-width: 16rem;
  font-size: 0.8rem;
  white-space: pre-line;
}
.row-actions {
  white-space: nowrap;
}
.overdue {
  color: var(--negative, #b91c1c);
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
}
.history-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}
.history-header h3 {
  margin: 0;
}
.history-header label {
  display: flex;
  align-items: center;
  font-size: 0.85rem;
  gap: 0.4rem;
}
</style>
