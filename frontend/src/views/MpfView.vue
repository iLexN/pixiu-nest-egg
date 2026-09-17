<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  api,
  ApiError,
  type MpfAccount,
  type MpfFigures,
  type MpfOverview,
} from '../api'
import { fmtMoney, fmtPercent, signClass } from '../format'
import RowActions from '../components/RowActions.vue'

const overview = ref<MpfOverview | null>(null)
const message = ref('')
const error = ref('')

async function load() {
  error.value = ''
  try {
    overview.value = await api.mpfOverview()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- account editing -----

interface AccountDraft {
  label: string
  trustee: string
  contributions: string
  balance: string
  plan_name: string
  member_no: string
}

const editing = ref<MpfAccount | 'new' | null>(null)
const draft = ref<AccountDraft>({
  label: '',
  trustee: '',
  contributions: '',
  balance: '',
  plan_name: '',
  member_no: '',
})

function startEdit(account: MpfAccount | 'new') {
  editing.value = account
  draft.value =
    account === 'new'
      ? {
          label: '',
          trustee: '',
          contributions: '',
          balance: '',
          plan_name: '',
          member_no: '',
        }
      : {
          label: account.label,
          trustee: account.trustee ?? '',
          contributions: String(account.contributions),
          balance: String(account.balance),
          plan_name: account.plan_name ?? '',
          member_no: account.member_no ?? '',
        }
}

function parseAmount(field: 'contributions' | 'balance'): number | null {
  const text = draft.value[field].trim()
  const parsed = Number(text)
  if (text === '' || !Number.isFinite(parsed)) {
    error.value = `${field === 'contributions' ? '總供款額' : '帳戶結存'}必須是數字`
    return null
  }
  return parsed
}

async function saveAccount() {
  const contributions = parseAmount('contributions')
  const balance = parseAmount('balance')
  if (contributions === null || balance === null) return
  const body = {
    label: draft.value.label.trim(),
    trustee: draft.value.trustee.trim() || null,
    contributions,
    balance,
    plan_name: draft.value.plan_name.trim() || null,
    member_no: draft.value.member_no.trim() || null,
  }
  try {
    if (editing.value === 'new') {
      await api.createMpfAccount(body)
      message.value = '已新增 MPF 帳戶'
    } else if (editing.value) {
      await api.updateMpfAccount(editing.value.id, body)
      message.value = '已儲存 MPF 帳戶'
    }
    editing.value = null
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeAccount(account: MpfAccount) {
  if (!window.confirm(`刪除 MPF 帳戶 ${account.label}？`)) return
  try {
    await api.deleteMpfAccount(account.id)
    message.value = `已刪除 ${account.label}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- note -----

const noteDraft = ref('')
const noteLoaded = ref(false)

function startNote() {
  noteDraft.value = overview.value?.note ?? ''
  noteLoaded.value = true
}

async function saveNote() {
  try {
    await api.updateMpfNote(noteDraft.value.trim() || null)
    noteLoaded.value = false
    message.value = '已儲存備註'
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- history -----

const accountLabel = computed(() => {
  const map = new Map<number, string>()
  for (const account of overview.value?.accounts ?? []) map.set(account.id, account.label)
  return map
})

const historyRows = computed(() =>
  [...(overview.value?.history ?? [])].sort((a, b) =>
    b.recorded_on.localeCompare(a.recorded_on),
  ),
)

async function removeHistory(id: number) {
  if (!window.confirm('刪除這筆記錄？上月/最高數字會重新計算。')) return
  try {
    await api.deleteMpfHistory(id)
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function fmtFigures(pair: MpfFigures | null): string {
  if (!pair) return '—'
  const rate = fmtPercent(pair.rate) || '—'
  return `${rate} / ${fmtMoney(pair.gain)}`
}

/** Green when `value` beats `reference` (now vs. last month), red when worse. */
function compareClass(
  value: number | null | undefined,
  reference: number | null | undefined,
): string {
  if (value === null || value === undefined || reference === null || reference === undefined) {
    return ''
  }
  return signClass(value - reference)
}

onMounted(load)
</script>

<template>
  <section>
    <div v-if="overview" class="card">
      <div class="totals-grid" aria-label="MPF 合計">
        <div class="total-card">
          <span>總供款額</span>
          <strong>{{ fmtMoney(overview.totals.buy) }}</strong>
        </div>
        <div class="total-card">
          <span>帳戶結存</span>
          <strong>{{ fmtMoney(overview.totals.now) }}</strong>
        </div>
        <div class="total-card">
          <span>回報率</span>
          <strong :class="signClass(overview.totals.rate)">
            {{ fmtPercent(overview.totals.rate) }}
          </strong>
        </div>
        <div class="total-card">
          <span>淨收益</span>
          <strong :class="signClass(overview.totals.gain)">
            {{ fmtMoney(overview.totals.gain) }}
          </strong>
        </div>
        <div class="total-card">
          <span>上月</span>
          <strong>
            <template v-if="overview.totals.last_month">
              <span
                :class="compareClass(overview.totals.rate, overview.totals.last_month.rate)"
                >{{ fmtPercent(overview.totals.last_month.rate) || '—' }}</span
              >
              /
              <span
                :class="compareClass(overview.totals.gain, overview.totals.last_month.gain)"
                >{{ fmtMoney(overview.totals.last_month.gain) }}</span
              >
            </template>
            <template v-else>—</template>
          </strong>
        </div>
        <div class="total-card">
          <span>最高</span>
          <strong>{{ fmtFigures(overview.totals.max) }}</strong>
        </div>
      </div>

      <table>
        <thead>
          <tr>
            <th>帳戶</th>
            <th>受託人</th>
            <th class="num">總供款額</th>
            <th class="num">帳戶結存</th>
            <th class="num">回報率</th>
            <th class="num">淨收益</th>
            <th class="num">上月</th>
            <th class="num">最高</th>
            <th>計劃名稱</th>
            <th>成員編號</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="account in overview.accounts" :key="account.id">
            <td>{{ account.label }}</td>
            <td>{{ account.trustee ?? '' }}</td>
            <td class="num">{{ fmtMoney(account.contributions) }}</td>
            <td class="num">{{ fmtMoney(account.balance) }}</td>
            <td class="num" :class="signClass(account.rate)">{{ fmtPercent(account.rate) }}</td>
            <td class="num" :class="signClass(account.gain)">{{ fmtMoney(account.gain) }}</td>
            <td class="num">
              <template v-if="account.last_month">
                <span :class="compareClass(account.rate, account.last_month.rate)">{{
                  fmtPercent(account.last_month.rate) || '—'
                }}</span>
                /
                <span :class="compareClass(account.gain, account.last_month.gain)">{{
                  fmtMoney(account.last_month.gain)
                }}</span>
              </template>
              <template v-else>—</template>
            </td>
            <td class="num">{{ fmtFigures(account.max) }}</td>
            <td class="note">{{ account.plan_name ?? '' }}</td>
            <td>{{ account.member_no ?? '' }}</td>
            <td class="row-actions">
              <RowActions @edit="startEdit(account)" @remove="removeAccount(account)" />
            </td>
          </tr>
          <tr v-if="overview.accounts.length === 0">
            <td colspan="11" class="muted">沒有 MPF 帳戶 — 執行匯入或新增帳戶</td>
          </tr>
        </tbody>
      </table>

      <button type="button" class="link" @click="startEdit('new')">新增帳戶</button>

      <form v-if="editing" class="account-form" @submit.prevent="saveAccount">
        <h4>{{ editing === 'new' ? '新增帳戶' : `編輯 ${editing.label}` }}</h4>
        <label>
          帳戶
          <input v-model="draft.label" required />
        </label>
        <label>
          受託人
          <input v-model="draft.trustee" />
        </label>
        <label>
          總供款額
          <input v-model="draft.contributions" type="number" step="any" required />
        </label>
        <label>
          帳戶結存
          <input v-model="draft.balance" type="number" step="any" required />
        </label>
        <label>
          計劃名稱
          <input v-model="draft.plan_name" />
        </label>
        <label>
          成員編號
          <input v-model="draft.member_no" />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editing = null">取消</button>
        </div>
      </form>
    </div>

    <div v-if="overview" class="card">
      <h3>備註</h3>
      <template v-if="noteLoaded">
        <textarea v-model="noteDraft" rows="3"></textarea>
        <div>
          <button type="button" @click="saveNote">儲存</button>
          <button type="button" class="link" @click="noteLoaded = false">取消</button>
        </div>
      </template>
      <p v-else class="note-text" @click="startNote">
        {{ overview.note || '按此新增備註' }}
      </p>
    </div>

    <div v-if="overview && historyRows.length" class="card">
      <h3>記錄</h3>
      <table>
        <thead>
          <tr>
            <th>日期</th>
            <th>帳戶</th>
            <th class="num">總供款額</th>
            <th class="num">帳戶結存</th>
            <th class="num">回報率</th>
            <th class="num">淨收益</th>
            <th></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in historyRows" :key="row.id">
            <td>{{ row.recorded_on }}</td>
            <td>{{ accountLabel.get(row.account_id) ?? `#${row.account_id}` }}</td>
            <td class="num">{{ fmtMoney(row.contributions) }}</td>
            <td class="num">{{ fmtMoney(row.balance) }}</td>
            <td class="num" :class="signClass(row.rate)">{{ fmtPercent(row.rate) }}</td>
            <td class="num" :class="signClass(row.gain)">{{ fmtMoney(row.gain) }}</td>
            <td><span v-if="row.synthetic" class="muted">月末補記</span></td>
            <td class="row-actions">
              <button type="button" class="link danger" @click="removeHistory(row.id)">
                刪除
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="card hints">
      <h3>手動步驟提醒</h3>
      <p class="muted">
        Overview!B6 仍讀取試算表 MPF!B2 — 此表更新後該格已凍結，Overview、Month
        Stat 等尚未遷移的章節仍以試算表為準。
      </p>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
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
.account-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
}
.account-form h4 {
  grid-column: 1 / -1;
  margin: 0;
}
.account-form label {
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
.note-text {
  white-space: pre-line;
  cursor: pointer;
}
.note-text:empty::before {
  content: '';
}
textarea {
  width: 100%;
}
.hints p {
  margin: 0;
}
</style>
