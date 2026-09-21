<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  api,
  ApiError,
  type AiaEvent,
  type AiaPolicy,
  type AiaPolicyWithEvents,
  type AiaSummary,
} from '../api'
import { fmtMoney, fmtPercent, signClass, todayIso } from '../format'
import DateInput from '../components/DateInput.vue'
import RowActions from '../components/RowActions.vue'

const summary = ref<AiaSummary | null>(null)
const message = ref('')
const error = ref('')

async function load() {
  error.value = ''
  try {
    summary.value = await api.aiaSummary()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

/** Next-pay dates are month-level; the day is always the 1st. */
function fmtMonth(value: string | null | undefined): string {
  return value?.slice(0, 7) ?? ''
}

function flagLabel(policy: AiaPolicy): string {
  if (policy.excluded) return '不計入'
  if (!policy.in_account) return '非戶口'
  return ''
}

// ----- USD→HKD rate -----

const editingRate = ref(false)
const rateDraft = ref<string | number>('')

function startRateEdit() {
  rateDraft.value = summary.value?.rate ?? ''
  editingRate.value = true
}

async function saveRate() {
  const rate = numOrNull(rateDraft.value)
  if (rate === null || rate <= 0) {
    error.value = '匯率必須是正數'
    return
  }
  try {
    await api.updateAiaRate(rate)
    editingRate.value = false
    message.value = '已更新匯率'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- policy add/edit form -----

interface PolicyDraft {
  label: string
  policy_no: string
  next_pay_date: string
  premium_usd: string | number
  value_usd: string | number
  remaining_years: string | number
  withdrew_usd: string | number
  note: string
  link: string
  excluded: boolean
  in_account: boolean
}

const emptyPolicyDraft: PolicyDraft = {
  label: '',
  policy_no: '',
  next_pay_date: '',
  premium_usd: '',
  value_usd: '',
  remaining_years: '',
  withdrew_usd: '',
  note: '',
  link: '',
  excluded: false,
  in_account: true,
}

const editingPolicy = ref<AiaPolicy | 'new' | null>(null)
const policyDraft = ref<PolicyDraft>({ ...emptyPolicyDraft })

function startPolicyEdit(policy: AiaPolicy | 'new') {
  editingPolicy.value = policy
  policyDraft.value =
    policy === 'new'
      ? { ...emptyPolicyDraft }
      : {
          label: policy.label,
          policy_no: policy.policy_no ?? '',
          next_pay_date: policy.next_pay_date ?? '',
          premium_usd: String(policy.premium_usd),
          value_usd: String(policy.value_usd),
          remaining_years: policy.remaining_years === null ? '' : String(policy.remaining_years),
          withdrew_usd: String(policy.withdrew_usd),
          note: policy.note ?? '',
          link: policy.link ?? '',
          excluded: policy.excluded,
          in_account: policy.in_account,
        }
}

async function savePolicy() {
  const premium = numOrNull(policyDraft.value.premium_usd)
  const value = numOrNull(policyDraft.value.value_usd)
  const withdrew = numOrNull(policyDraft.value.withdrew_usd)
  if (premium === null || value === null || withdrew === null) {
    error.value = '已繳保費、現值及已提取必須是數字'
    return
  }
  const body = {
    label: policyDraft.value.label.trim(),
    policy_no: policyDraft.value.policy_no.trim() || null,
    next_pay_date: policyDraft.value.next_pay_date || null,
    premium_usd: premium,
    value_usd: value,
    remaining_years: numOrNull(policyDraft.value.remaining_years),
    withdrew_usd: withdrew,
    note: policyDraft.value.note.trim() || null,
    link: policyDraft.value.link.trim() || null,
    excluded: policyDraft.value.excluded,
    in_account: policyDraft.value.in_account,
  }
  try {
    if (editingPolicy.value === 'new') {
      await api.createAiaPolicy(body)
      message.value = '已新增保單'
    } else if (editingPolicy.value) {
      await api.updateAiaPolicy(editingPolicy.value.id, body)
      message.value = '已儲存保單'
    }
    editingPolicy.value = null
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removePolicy(policy: AiaPolicy) {
  if (!window.confirm(`刪除保單 ${policy.label}？其繳費/提取記錄會一併刪除。`)) return
  try {
    await api.deleteAiaPolicy(policy.id)
    message.value = `已刪除 ${policy.label}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- 繳費 / 提取 event forms -----

const paying = ref<{
  policy: AiaPolicy
  event_date: string
  amount: string | number
  next_pay_date: string
  note: string
} | null>(null)

/** The form's suggested next due date: current date +1 year. */
function defaultNextPay(policy: AiaPolicy): string {
  if (!policy.next_pay_date) return ''
  const date = new Date(`${policy.next_pay_date}T00:00:00`)
  date.setFullYear(date.getFullYear() + 1)
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

function startPay(policy: AiaPolicy) {
  paying.value = {
    policy,
    event_date: todayIso(),
    amount: '',
    next_pay_date: defaultNextPay(policy),
    note: '',
  }
}

async function savePay() {
  const current = paying.value
  if (!current) return
  const amount = numOrNull(current.amount)
  if (amount === null || amount <= 0) {
    error.value = '金額必須是正數'
    return
  }
  try {
    await api.createAiaEvent({
      policy_id: current.policy.id,
      kind: 'payment',
      event_date: current.event_date,
      amount_usd: amount,
      next_pay_date: current.next_pay_date || null,
      note: current.note.trim() || null,
    })
    paying.value = null
    message.value = '已記錄繳費 — 已繳保費、餘下年期及下期繳費日已更新'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

const withdrawing = ref<{
  policy: AiaPolicy
  event_date: string
  amount: string | number
  note: string
} | null>(null)

function startWithdraw(policy: AiaPolicy) {
  withdrawing.value = { policy, event_date: todayIso(), amount: '', note: '' }
}

async function saveWithdraw() {
  const current = withdrawing.value
  if (!current) return
  const amount = numOrNull(current.amount)
  if (amount === null || amount <= 0) {
    error.value = '金額必須是正數'
    return
  }
  try {
    await api.createAiaEvent({
      policy_id: current.policy.id,
      kind: 'withdrawal',
      event_date: current.event_date,
      amount_usd: amount,
      note: current.note.trim() || null,
    })
    withdrawing.value = null
    message.value = '已記錄提取'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeEvent(policy: AiaPolicyWithEvents, event: AiaEvent) {
  const kind = event.kind === 'payment' ? '繳費' : '提取'
  if (!window.confirm(`刪除 ${event.event_date} 的${kind}記錄？保單數值會還原。`)) return
  try {
    await api.deleteAiaEvent(event.id)
    message.value = `已刪除 ${policy.label} ${event.event_date} ${kind}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function policiesWithEvents(): AiaPolicyWithEvents[] {
  return (summary.value?.policies ?? []).filter((policy) => policy.events.length > 0)
}

onMounted(load)
</script>

<template>
  <section>
    <div v-if="summary" class="card">
      <div class="totals-grid" aria-label="AIA 合計">
        <div class="total-card">
          <span>總保費 USD</span>
          <strong>{{ fmtMoney(summary.totals.premium) }}</strong>
          <small v-if="summary.totals.premium_hkd !== null" class="muted">
            HKD {{ fmtMoney(summary.totals.premium_hkd) }}
          </small>
        </div>
        <div class="total-card">
          <span>現值 USD</span>
          <strong>{{ fmtMoney(summary.totals.value) }}</strong>
          <small v-if="summary.totals.value_hkd !== null" class="muted">
            HKD {{ fmtMoney(summary.totals.value_hkd) }}
          </small>
        </div>
        <div class="total-card">
          <span>已提取 USD</span>
          <strong>{{ fmtMoney(summary.totals.withdrew) }}</strong>
          <small v-if="summary.totals.withdrew_hkd !== null" class="muted">
            HKD {{ fmtMoney(summary.totals.withdrew_hkd) }}
          </small>
        </div>
        <div class="total-card">
          <span>回報率</span>
          <strong :class="signClass(summary.totals.balance_pct)">
            {{ fmtPercent(summary.totals.balance_pct) || '—' }}
          </strong>
        </div>
        <div class="total-card">
          <span>淨變動 HKD</span>
          <strong :class="signClass(summary.totals.net_change_hkd)">
            {{ fmtMoney(summary.totals.net_change_hkd) || '—' }}
          </strong>
          <small class="muted">現值 − 保費 − 提取</small>
        </div>
        <div class="total-card">
          <span>AIA 戶口顯示值</span>
          <strong>{{ fmtMoney(summary.totals.display_value) }}</strong>
          <small class="muted">對照 AIA 戶口</small>
        </div>
        <div class="total-card">
          <span>下期繳費</span>
          <strong>{{ fmtMonth(summary.next_premium_due) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>USD → HKD</span>
          <strong>{{ summary.rate ?? '未設定' }}</strong>
          <button type="button" class="link" @click="startRateEdit">編輯</button>
        </div>
      </div>

      <table>
        <thead>
          <tr>
            <th>名稱</th>
            <th>保單號</th>
            <th>下期繳費</th>
            <th class="num">已繳保費</th>
            <th class="num">現值</th>
            <th class="num">回報率</th>
            <th class="num">餘下年期</th>
            <th class="num">已提取</th>
            <th></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="policy in summary.policies" :key="policy.id">
            <td>
              {{ policy.label }}
              <small v-if="policy.note" class="muted note">{{ policy.note }}</small>
            </td>
            <td>{{ policy.policy_no ?? '' }}</td>
            <td>{{ fmtMonth(policy.next_pay_date) }}</td>
            <td class="num">{{ fmtMoney(policy.premium_usd) }}</td>
            <td class="num">{{ fmtMoney(policy.value_usd) }}</td>
            <td class="num" :class="signClass(policy.balance_pct)">
              {{ fmtPercent(policy.balance_pct) || '—' }}
            </td>
            <td class="num">{{ policy.remaining_years ?? '' }}</td>
            <td class="num">{{ fmtMoney(policy.withdrew_usd) || '—' }}</td>
            <td>
              <span v-if="flagLabel(policy)" class="muted flag">{{ flagLabel(policy) }}</span>
              <a
                v-if="policy.link"
                :href="policy.link"
                target="_blank"
                rel="noopener"
                class="pdf-link"
                >PDF</a
              >
            </td>
            <td class="row-actions">
              <button type="button" class="link" @click="startPay(policy)">繳費</button>
              <button type="button" class="link" @click="startWithdraw(policy)">提取</button>
              <RowActions @edit="startPolicyEdit(policy)" @remove="removePolicy(policy)" />
            </td>
          </tr>
          <tr v-if="summary.policies.length === 0">
            <td colspan="10" class="muted">沒有保單 — 執行匯入或新增保單</td>
          </tr>
        </tbody>
      </table>
      <button type="button" class="link" @click="startPolicyEdit('new')">新增保單</button>

      <form v-if="editingPolicy" class="inline-form" @submit.prevent="savePolicy">
        <h4>{{ editingPolicy === 'new' ? '新增保單' : `編輯 ${editingPolicy.label}` }}</h4>
        <label>
          名稱
          <input v-model="policyDraft.label" required />
        </label>
        <label>
          保單號
          <input v-model="policyDraft.policy_no" placeholder="B632611401" />
        </label>
        <label>
          下期繳費
          <DateInput v-model="policyDraft.next_pay_date" />
        </label>
        <label>
          已繳保費 USD
          <input v-model="policyDraft.premium_usd" type="number" step="any" required />
        </label>
        <label>
          現值 USD
          <input v-model="policyDraft.value_usd" type="number" step="any" required />
        </label>
        <label>
          餘下年期
          <input v-model="policyDraft.remaining_years" type="number" step="any" />
        </label>
        <label>
          已提取 USD
          <input v-model="policyDraft.withdrew_usd" type="number" step="any" required />
        </label>
        <label>
          連結
          <input v-model="policyDraft.link" placeholder="https://" />
        </label>
        <label>
          備註
          <input v-model="policyDraft.note" />
        </label>
        <label class="check">
          <input v-model="policyDraft.excluded" type="checkbox" />
          不計入合計（他人的份額）
        </label>
        <label class="check">
          <input v-model="policyDraft.in_account" type="checkbox" />
          計入 AIA 戶口顯示值
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingPolicy = null">取消</button>
        </div>
      </form>

      <form v-if="editingRate" class="inline-form" @submit.prevent="saveRate">
        <h4>USD → HKD 匯率</h4>
        <label>
          匯率
          <input v-model="rateDraft" type="number" step="any" inputmode="decimal" required />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingRate = false">取消</button>
        </div>
      </form>

      <form v-if="paying" class="inline-form" @submit.prevent="savePay">
        <h4>繳費 — {{ paying.policy.label }} {{ paying.policy.policy_no ?? '' }}</h4>
        <label>
          日期
          <DateInput v-model="paying.event_date" required />
        </label>
        <label>
          金額 USD
          <input v-model="paying.amount" type="number" step="any" inputmode="decimal" required />
        </label>
        <label>
          新下期繳費
          <DateInput v-model="paying.next_pay_date" />
        </label>
        <label>
          備註
          <input v-model="paying.note" />
        </label>
        <div class="form-actions">
          <button type="submit">繳費</button>
          <button type="button" class="link" @click="paying = null">取消</button>
        </div>
      </form>

      <form v-if="withdrawing" class="inline-form" @submit.prevent="saveWithdraw">
        <h4>提取 — {{ withdrawing.policy.label }} {{ withdrawing.policy.policy_no ?? '' }}</h4>
        <label>
          日期
          <DateInput v-model="withdrawing.event_date" required />
        </label>
        <label>
          金額 USD
          <input v-model="withdrawing.amount" type="number" step="any" inputmode="decimal" required />
        </label>
        <label>
          備註
          <input v-model="withdrawing.note" />
        </label>
        <div class="form-actions">
          <button type="submit">提取</button>
          <button type="button" class="link" @click="withdrawing = null">取消</button>
        </div>
      </form>
    </div>

    <div v-if="policiesWithEvents().length" class="card">
      <h3>記錄</h3>
      <div v-for="policy in policiesWithEvents()" :key="policy.id" class="event-block">
        <h4>
          {{ policy.label }}
          <small v-if="policy.policy_no" class="muted">{{ policy.policy_no }}</small>
        </h4>
        <table>
          <thead>
            <tr>
              <th>日期</th>
              <th>類別</th>
              <th class="num">金額 USD</th>
              <th>備註</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="event in policy.events" :key="event.id">
              <td>{{ event.event_date }}</td>
              <td>{{ event.kind === 'payment' ? '繳費' : '提取' }}</td>
              <td class="num">{{ fmtMoney(event.amount_usd) }}</td>
              <td>{{ event.note ?? '' }}</td>
              <td class="row-actions">
                <button
                  type="button"
                  class="link danger"
                  @click="removeEvent(policy, event)"
                >
                  刪除
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="card hints">
      <h3>手動步驟提醒</h3>
      <p class="muted">
        在此記錄繳費會同時更新已繳保費、餘下年期及下期繳費日 — 毋須逐項修改。
        試算表的 AIA 表不會同步更新；USD→HKD 匯率是 Overview!N3 的手動副本，
        試算表更新匯率後請在此修改。Overview、Month Stat 等尚未遷移的章節仍以試算表為準。
      </p>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
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
.total-card small {
  display: block;
  font-size: 0.75rem;
}
.flag {
  font-size: 0.75rem;
}
.note {
  display: block;
  font-size: 0.75rem;
  white-space: normal;
}
.pdf-link {
  margin-left: 0.35rem;
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 600;
  text-decoration: none;
}
.pdf-link:hover {
  text-decoration: underline;
}
.event-block {
  margin-top: 0.75rem;
}
.event-block h4 {
  margin: 0.25rem 0;
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
.inline-form label.check {
  flex-direction: row;
  align-items: center;
  gap: 0.4rem;
}
.form-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: 0.5rem;
}
.hints p {
  margin: 0;
}
</style>
