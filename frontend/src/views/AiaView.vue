<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  api,
  type AiaEvent,
  type AiaPolicy,
  type AiaPolicyWithEvents,
  type AiaSummary,
} from '../api'
import { fmtMoney, fmtMonth, fmtPercent, signClass, shiftYearIso, todayIso } from '../format'
import { numOrNull } from '../parse'
import DateInput from '../components/DateInput.vue'
import InlineForm from '../components/InlineForm.vue'
import RowActions from '../components/RowActions.vue'
import { usePage } from '../page'

const summary = ref<AiaSummary | null>(null)

const { message, error, reload, run } = usePage(async () => {
  summary.value = await api.aiaSummary()
})

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

function saveRate() {
  run(async () => {
    const rate = numOrNull(rateDraft.value)
    if (rate === null || rate <= 0) {
      throw new Error('匯率必須是正數')
    }
    await api.updateFxRate('USD', rate)
    editingRate.value = false
  }, { message: '已更新匯率' })
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

function savePolicy() {
  const wasNew = editingPolicy.value === 'new'
  const target = editingPolicy.value
  run(async () => {
    const premium = numOrNull(policyDraft.value.premium_usd)
    const value = numOrNull(policyDraft.value.value_usd)
    const withdrew = numOrNull(policyDraft.value.withdrew_usd)
    if (premium === null || value === null || withdrew === null) {
      throw new Error('已繳保費、現值及已提取必須是數字')
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
    if (wasNew) {
      await api.createAiaPolicy(body)
    } else if (target && target !== 'new') {
      await api.updateAiaPolicy(target.id, body)
    }
    editingPolicy.value = null
  }, {
    message: wasNew ? '已新增保單' : '已儲存保單',
  })
}

function removePolicy(policy: AiaPolicy) {
  run(() => api.deleteAiaPolicy(policy.id), {
    confirm: `刪除保單 ${policy.label}？其繳費/提取記錄會一併刪除。`,
    message: `已刪除 ${policy.label}`,
  })
}

// ----- 繳費 / 提取 event forms -----

const paying = ref<{
  policy: AiaPolicy
  event_date: string
  amount: string | number
  next_pay_date: string
  note: string
} | null>(null)

/** The form's suggested next due date: current date +1 year. Next-pay
 * dates are month-level; the day is always the 1st. */
function defaultNextPay(policy: AiaPolicy): string {
  return policy.next_pay_date ? shiftYearIso(policy.next_pay_date, 1) : ''
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

function savePay() {
  const current = paying.value
  if (!current) return
  run(async () => {
    const amount = numOrNull(current.amount)
    if (amount === null || amount <= 0) {
      throw new Error('金額必須是正數')
    }
    await api.createAiaEvent({
      policy_id: current.policy.id,
      kind: 'payment',
      event_date: current.event_date,
      amount_usd: amount,
      next_pay_date: current.next_pay_date || null,
      note: current.note.trim() || null,
    })
    paying.value = null
  }, { message: '已記錄繳費 — 已繳保費、餘下年期及下期繳費日已更新' })
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

function saveWithdraw() {
  const current = withdrawing.value
  if (!current) return
  run(async () => {
    const amount = numOrNull(current.amount)
    if (amount === null || amount <= 0) {
      throw new Error('金額必須是正數')
    }
    await api.createAiaEvent({
      policy_id: current.policy.id,
      kind: 'withdrawal',
      event_date: current.event_date,
      amount_usd: amount,
      note: current.note.trim() || null,
    })
    withdrawing.value = null
  }, { message: '已記錄提取' })
}

function removeEvent(policy: AiaPolicyWithEvents, event: AiaEvent) {
  const kind = event.kind === 'payment' ? '繳費' : '提取'
  run(() => api.deleteAiaEvent(event.id), {
    confirm: `刪除 ${event.event_date} 的${kind}記錄？保單數值會還原。`,
    message: `已刪除 ${policy.label} ${event.event_date} ${kind}`,
  })
}

function policiesWithEvents(): AiaPolicyWithEvents[] {
  return (summary.value?.policies ?? []).filter((policy) => policy.events.length > 0)
}

// ----- note -----

const noteDraft = ref('')
const noteLoaded = ref(false)

function startNote() {
  noteDraft.value = summary.value?.note ?? ''
  noteLoaded.value = true
}

function saveNote() {
  run(async () => {
    await api.updateAiaNote(noteDraft.value.trim() || null)
    noteLoaded.value = false
  }, { message: '已儲存備註' })
}

onMounted(reload)
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

      <InlineForm
        v-if="editingPolicy"
        :title="editingPolicy === 'new' ? '新增保單' : `編輯 ${editingPolicy.label}`"
        @submit="savePolicy"
        @cancel="editingPolicy = null"
      >
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
      </InlineForm>

      <InlineForm
        v-if="editingRate"
        title="USD → HKD 匯率"
        @submit="saveRate"
        @cancel="editingRate = false"
      >
        <label>
          匯率
          <input v-model="rateDraft" type="number" step="any" inputmode="decimal" required />
        </label>
      </InlineForm>

      <InlineForm
        v-if="paying"
        :title="`繳費 — ${paying.policy.label} ${paying.policy.policy_no ?? ''}`"
        submit-label="繳費"
        @submit="savePay"
        @cancel="paying = null"
      >
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
      </InlineForm>

      <InlineForm
        v-if="withdrawing"
        :title="`提取 — ${withdrawing.policy.label} ${withdrawing.policy.policy_no ?? ''}`"
        submit-label="提取"
        @submit="saveWithdraw"
        @cancel="withdrawing = null"
      >
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
      </InlineForm>
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

    <div v-if="summary" class="card">
      <h3>備註</h3>
      <template v-if="noteLoaded">
        <textarea v-model="noteDraft" rows="3"></textarea>
        <div>
          <button type="button" @click="saveNote">儲存</button>
          <button type="button" class="link" @click="noteLoaded = false">取消</button>
        </div>
      </template>
      <p v-else class="note-text" @click="startNote">
        {{ summary.note || '按此新增備註' }}
      </p>
    </div>

    <div class="card hints">
      <h3>手動步驟提醒</h3>
      <p class="muted">
        在此記錄繳費會同時更新已繳保費、餘下年期及下期繳費日 — 毋須逐項修改。
        試算表的 AIA 表不會同步更新；USD→HKD 匯率是 Overview!N3 的手動副本，
        試算表更新匯率後請在此修改。Overview 等尚未遷移的章節仍以試算表為準。
      </p>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error?.message }}</p>
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
.event-block h4 {
  margin: 0.25rem 0;
}
.hints p {
  margin: 0;
}
</style>
