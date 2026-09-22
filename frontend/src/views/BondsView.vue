<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  api,
  ApiError,
  type Bond,
  type BondCoupon,
  type BondSummary,
  type ManualAsset,
} from '../api'
import { fmtMoney, fmtPercent, signClass, todayIso } from '../format'
import DateInput from '../components/DateInput.vue'
import RowActions from '../components/RowActions.vue'

const summary = ref<BondSummary | null>(null)
const assets = ref<ManualAsset[]>([])
const message = ref('')
const error = ref('')

const cashAssets = computed(() => assets.value.filter((asset) => asset.kind === 'cash'))

async function load() {
  error.value = ''
  try {
    const [summaryData, assetsData] = await Promise.all([
      api.bondSummary(),
      api.listManualAssets(),
    ])
    summary.value = summaryData
    assets.value = assetsData
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function couponStatusLabel(coupon: BondCoupon): string {
  if (coupon.status === 'RECEIVED') return '已收'
  if (coupon.status === 'PENDING_FIX') return '待定'
  return '待收'
}

// ----- bond form -----

interface BondDraft {
  label: string
  issue_no: string
  principal: string | number
  maturity_date: string
  note: string
}

const emptyBondDraft: BondDraft = {
  label: '',
  issue_no: '',
  principal: '',
  maturity_date: '',
  note: '',
}

const editingBond = ref<Bond | 'new' | null>(null)
const bondDraft = ref<BondDraft>({ ...emptyBondDraft })

function startBondEdit(bond: Bond | 'new') {
  editingBond.value = bond
  bondDraft.value =
    bond === 'new'
      ? { ...emptyBondDraft }
      : {
          label: bond.label,
          issue_no: bond.issue_no ?? '',
          principal: String(bond.principal),
          maturity_date: bond.maturity_date,
          note: bond.note ?? '',
        }
}

async function saveBond() {
  const principal = Number(bondDraft.value.principal)
  if (!Number.isFinite(principal)) {
    error.value = '本金必須是數字'
    return
  }
  const body = {
    label: bondDraft.value.label.trim(),
    issue_no: bondDraft.value.issue_no.trim() || null,
    principal,
    maturity_date: bondDraft.value.maturity_date,
    note: bondDraft.value.note.trim() || null,
  }
  try {
    if (editingBond.value === 'new') {
      await api.createBond(body)
      message.value = '已新增債券'
    } else if (editingBond.value) {
      await api.updateBond(editingBond.value.id, body)
      message.value = '已儲存債券'
    }
    editingBond.value = null
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeBond(bond: Bond) {
  if (!window.confirm(`刪除債券 ${bond.label}？其付息記錄會一併刪除。`)) return
  try {
    await api.deleteBond(bond.id)
    message.value = `已刪除 ${bond.label}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- coupon add/edit form -----

interface CouponDraft {
  pay_date: string
  fixing_date: string
  annual_rate: string | number
  per_10k: string | number
  received_amount: string | number
  note: string
}

const emptyCouponDraft: CouponDraft = {
  pay_date: '',
  fixing_date: '',
  annual_rate: '',
  per_10k: '',
  received_amount: '',
  note: '',
}

const editingCoupon = ref<{ bondId: number; coupon: BondCoupon | 'new' } | null>(null)
const couponDraft = ref<CouponDraft>({ ...emptyCouponDraft })

function startCouponEdit(bondId: number, coupon: BondCoupon | 'new') {
  editingCoupon.value = { bondId, coupon }
  couponDraft.value =
    coupon === 'new'
      ? { ...emptyCouponDraft }
      : {
          pay_date: coupon.pay_date,
          fixing_date: coupon.fixing_date ?? '',
          annual_rate: coupon.annual_rate === null ? '' : String(coupon.annual_rate * 100),
          per_10k: coupon.per_10k === null ? '' : String(coupon.per_10k),
          received_amount: coupon.received_amount === null ? '' : String(coupon.received_amount),
          note: coupon.note ?? '',
        }
}

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

async function saveCoupon() {
  const editing = editingCoupon.value
  if (!editing) return
  const ratePct = numOrNull(couponDraft.value.annual_rate)
  const body = {
    bond_id: editing.bondId,
    pay_date: couponDraft.value.pay_date,
    fixing_date: couponDraft.value.fixing_date || null,
    // The form takes 年息率 as a percent (4); the API stores the fraction.
    annual_rate: ratePct === null ? null : ratePct / 100,
    per_10k: numOrNull(couponDraft.value.per_10k),
    received_amount: numOrNull(couponDraft.value.received_amount),
    note: couponDraft.value.note.trim() || null,
  }
  try {
    if (editing.coupon === 'new') {
      await api.createCoupon(body)
      message.value = '已新增付息'
    } else {
      await api.updateCoupon(editing.coupon.id, body)
      message.value = '已儲存付息'
    }
    editingCoupon.value = null
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function removeCoupon(coupon: BondCoupon) {
  if (!window.confirm(`刪除 ${coupon.pay_date} 的付息記錄？`)) return
  try {
    await api.deleteCoupon(coupon.id)
    message.value = `已刪除 ${coupon.pay_date} 付息`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- 釐定息率 (待定 -> pending) -----

const fixing = ref<{ coupon: BondCoupon; fixing_date: string; rate: string; per_10k: string } | null>(
  null,
)

function startFix(coupon: BondCoupon) {
  fixing.value = {
    coupon,
    fixing_date: coupon.fixing_date ?? summary.value?.today ?? '',
    rate: '',
    per_10k: '',
  }
}

async function saveFix() {
  const current = fixing.value
  if (!current) return
  const ratePct = numOrNull(current.rate)
  const per10k = numOrNull(current.per_10k)
  if (ratePct === null || per10k === null) {
    error.value = '年息率及每1萬利息均須填寫'
    return
  }
  try {
    await api.updateCoupon(current.coupon.id, {
      fixing_date: current.fixing_date || null,
      annual_rate: ratePct / 100,
      per_10k: per10k,
    })
    fixing.value = null
    message.value = '已釐定息率'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- 收訖 (pending -> received) -----

const receiving = ref<{
  coupon: BondCoupon
  amount: string | number
  bank_in: boolean
} | null>(null)

function startReceive(coupon: BondCoupon) {
  receiving.value = { coupon, amount: coupon.expected ?? '', bank_in: true }
}

async function saveReceive() {
  const current = receiving.value
  if (!current) return
  const amount = numOrNull(current.amount)
  if (amount === null) {
    error.value = '實收金額必須是數字'
    return
  }
  try {
    await api.updateCoupon(current.coupon.id, {
      received_amount: amount,
      bank_in: current.bank_in,
    })
    receiving.value = null
    message.value = '已記錄收訖'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

// ----- bond 收訖 (principal return -> received, optional bank-in) -----

const receivingBond = ref<{
  bond: Bond
  received_at: string
  credit_asset_id: number | null
  credit_amount: string | number
} | null>(null)

function startBondReceive(bond: Bond) {
  receivingBond.value = {
    bond,
    received_at: todayIso(),
    credit_asset_id: null,
    credit_amount: bond.principal,
  }
}

async function saveBondReceive() {
  const current = receivingBond.value
  if (!current) return
  const creditAmount = numOrNull(current.credit_amount)
  if (current.credit_asset_id !== null && creditAmount === null) {
    error.value = '存入金額必須是數字'
    return
  }
  try {
    await api.receiveBond(current.bond.id, {
      received_at: current.received_at || undefined,
      credit_asset_id: current.credit_asset_id ?? undefined,
      credit_amount: creditAmount ?? undefined,
    })
    receivingBond.value = null
    message.value = '已記錄本金收訖'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function unreceiveBond(bond: Bond) {
  if (!window.confirm(`取消收訖 ${bond.label}？已存入的活期與月結調整項目會一併還原。`)) return
  try {
    await api.unreceiveBond(bond.id)
    message.value = `已取消收訖 ${bond.label}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <section>
    <div v-if="summary" class="card">
      <div class="totals-grid" aria-label="債券合計">
        <div class="total-card">
          <span>現有本金</span>
          <strong>{{ fmtMoney(summary.totals.active_principal) }}</strong>
        </div>
        <div class="total-card">
          <span>下次付息</span>
          <strong v-if="summary.upcoming_coupons.length">
            {{ summary.upcoming_coupons[0].pay_date }}
            <small class="muted">{{ summary.upcoming_coupons[0].bond_label }}</small>
          </strong>
          <strong v-else>—</strong>
        </div>
        <div class="total-card">
          <span>未收付息</span>
          <strong>{{ summary.upcoming_coupons.length }} 期</strong>
        </div>
      </div>

      <div v-for="bond in summary.active" :key="bond.id" class="bond-block">
        <div class="bond-head">
          <h3>
            {{ bond.label }}
            <small v-if="bond.issue_no" class="muted">{{ bond.issue_no }}</small>
          </h3>
          <span class="muted">
            本金 {{ fmtMoney(bond.principal) }} · 到期 {{ bond.maturity_date }}
            <template v-if="bond.received_at"> · 收訖 {{ bond.received_at }}</template>
          </span>
          <button
            v-if="!bond.received_at"
            type="button"
            class="link"
            @click="startBondReceive(bond)"
          >
            收訖
          </button>
          <button v-else type="button" class="link" @click="unreceiveBond(bond)">
            取消收訖
          </button>
          <RowActions @edit="startBondEdit(bond)" @remove="removeBond(bond)" />
        </div>

        <table>
          <thead>
            <tr>
              <th>付息日</th>
              <th>釐定日</th>
              <th class="num">年息率</th>
              <th class="num">每1萬利息</th>
              <th class="num">利息</th>
              <th class="num">實收</th>
              <th class="num">差異</th>
              <th>狀態</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="coupon in bond.coupons" :key="coupon.id">
              <td>{{ coupon.pay_date }}</td>
              <td>{{ coupon.fixing_date ?? '' }}</td>
              <td class="num">
                <template v-if="coupon.annual_rate !== null">{{
                  fmtPercent(coupon.annual_rate)
                }}</template>
                <span v-else class="muted">待定</span>
              </td>
              <td class="num">
                <template v-if="coupon.per_10k !== null">{{ fmtMoney(coupon.per_10k) }}</template>
                <span v-else class="muted">待定</span>
              </td>
              <td class="num">
                <template v-if="coupon.expected !== null">{{ fmtMoney(coupon.expected) }}</template>
                <span v-else class="muted">待定</span>
              </td>
              <td class="num">{{ fmtMoney(coupon.received_amount) || '—' }}</td>
              <td class="num" :class="signClass(coupon.variance)">
                {{ fmtMoney(coupon.variance) || '—' }}
              </td>
              <td>{{ couponStatusLabel(coupon) }}</td>
              <td class="row-actions">
                <button
                  v-if="coupon.status === 'PENDING_FIX'"
                  type="button"
                  class="link"
                  @click="startFix(coupon)"
                >
                  釐定
                </button>
                <RowActions
                  :receive="coupon.status === 'PENDING'"
                  @receive="startReceive(coupon)"
                  @edit="startCouponEdit(bond.id, coupon)"
                  @remove="removeCoupon(coupon)"
                />
              </td>
            </tr>
            <tr v-if="bond.coupons.length === 0">
              <td colspan="9" class="muted">沒有付息記錄</td>
            </tr>
          </tbody>
        </table>

        <button type="button" class="link" @click="startCouponEdit(bond.id, 'new')">
          新增付息
        </button>
      </div>

      <p v-if="summary.active.length === 0" class="muted">
        沒有現有債券 — 執行匯入或新增債券
      </p>
      <button type="button" class="link" @click="startBondEdit('new')">新增債券</button>

      <form v-if="editingBond" class="inline-form" @submit.prevent="saveBond">
        <h4>{{ editingBond === 'new' ? '新增債券' : `編輯 ${editingBond.label}` }}</h4>
        <label>
          名稱
          <input v-model="bondDraft.label" required />
        </label>
        <label>
          發行編號
          <input v-model="bondDraft.issue_no" placeholder="03GB2710R" />
        </label>
        <label>
          本金
          <input v-model="bondDraft.principal" type="number" step="any" required />
        </label>
        <label>
          到期日
          <DateInput v-model="bondDraft.maturity_date" required />
        </label>
        <label>
          備註
          <input v-model="bondDraft.note" />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingBond = null">取消</button>
        </div>
      </form>

      <form v-if="editingCoupon" class="inline-form" @submit.prevent="saveCoupon">
        <h4>{{ editingCoupon.coupon === 'new' ? '新增付息' : '編輯付息' }}</h4>
        <label>
          付息日
          <DateInput v-model="couponDraft.pay_date" required />
        </label>
        <label>
          釐定日
          <DateInput v-model="couponDraft.fixing_date" />
        </label>
        <label>
          年息率（%）
          <input
            v-model="couponDraft.annual_rate"
            type="number"
            step="any"
            inputmode="decimal"
            placeholder="4"
          />
        </label>
        <label>
          每1萬利息
          <input v-model="couponDraft.per_10k" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          實收金額
          <input
            v-model="couponDraft.received_amount"
            type="number"
            step="any"
            inputmode="decimal"
          />
        </label>
        <label>
          備註
          <input v-model="couponDraft.note" />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingCoupon = null">取消</button>
        </div>
      </form>

      <form v-if="fixing" class="inline-form" @submit.prevent="saveFix">
        <h4>釐定息率 — {{ fixing.coupon.pay_date }}</h4>
        <label>
          釐定日
          <DateInput v-model="fixing.fixing_date" />
        </label>
        <label>
          年息率（%）
          <input v-model="fixing.rate" type="number" step="any" inputmode="decimal" required />
        </label>
        <label>
          每1萬利息
          <input v-model="fixing.per_10k" type="number" step="any" inputmode="decimal" required />
        </label>
        <div class="form-actions">
          <button type="submit">釐定</button>
          <button type="button" class="link" @click="fixing = null">取消</button>
        </div>
      </form>

      <form v-if="receiving" class="inline-form" @submit.prevent="saveReceive">
        <h4>收訖 — {{ receiving.coupon.pay_date }}</h4>
        <label>
          實收金額
          <input v-model="receiving.amount" type="number" step="any" inputmode="decimal" required />
        </label>
        <label class="checkbox">
          <input v-model="receiving.bank_in" type="checkbox" />
          存入活期 HS
        </label>
        <div class="form-actions">
          <button type="submit">收訖</button>
          <button type="button" class="link" @click="receiving = null">取消</button>
        </div>
      </form>

      <form v-if="receivingBond" class="inline-form" @submit.prevent="saveBondReceive">
        <h4>
          本金收訖 — {{ receivingBond.bond.label }}（{{ receivingBond.bond.maturity_date }} 到期）
        </h4>
        <label>
          收訖日
          <input v-model="receivingBond.received_at" type="date" required />
        </label>
        <label>
          存入活期
          <select v-model="receivingBond.credit_asset_id">
            <option :value="null">不存入</option>
            <option v-for="asset in cashAssets" :key="asset.id" :value="asset.id">
              {{ asset.label }}（{{ fmtMoney(asset.amount) }}）
            </option>
          </select>
        </label>
        <label v-if="receivingBond.credit_asset_id !== null">
          存入金額
          <input
            v-model="receivingBond.credit_amount"
            type="number"
            step="any"
            inputmode="decimal"
          />
        </label>
        <div class="form-actions">
          <button type="submit">收訖</button>
          <button type="button" class="link" @click="receivingBond = null">取消</button>
        </div>
      </form>
    </div>

    <div v-if="summary && summary.matured.length" class="card">
      <h3>已到期</h3>

      <div v-for="bond in summary.matured" :key="bond.id" class="bond-block">
        <div class="bond-head">
          <h3>
            {{ bond.label }}
            <small v-if="bond.issue_no" class="muted">{{ bond.issue_no }}</small>
          </h3>
          <span class="muted">
            本金 {{ fmtMoney(bond.principal) }} · 到期 {{ bond.maturity_date }}
            <template v-if="bond.received_at"> · 收訖 {{ bond.received_at }}</template>
            <template v-else> · <span class="unreceived">本金未收</span></template>
          </span>
          <button
            v-if="!bond.received_at"
            type="button"
            class="link"
            @click="startBondReceive(bond)"
          >
            收訖
          </button>
          <button v-else type="button" class="link" @click="unreceiveBond(bond)">
            取消收訖
          </button>
          <RowActions @edit="startBondEdit(bond)" @remove="removeBond(bond)" />
        </div>
        <table v-if="bond.coupons.length">
          <thead>
            <tr>
              <th>付息日</th>
              <th class="num">年息率</th>
              <th class="num">利息</th>
              <th class="num">實收</th>
              <th class="num">差異</th>
              <th>狀態</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="coupon in bond.coupons" :key="coupon.id">
              <td>{{ coupon.pay_date }}</td>
              <td class="num">
                <template v-if="coupon.annual_rate !== null">{{
                  fmtPercent(coupon.annual_rate)
                }}</template>
                <span v-else class="muted">待定</span>
              </td>
              <td class="num">
                <template v-if="coupon.expected !== null">{{ fmtMoney(coupon.expected) }}</template>
                <span v-else class="muted">待定</span>
              </td>
              <td class="num">{{ fmtMoney(coupon.received_amount) || '—' }}</td>
              <td class="num" :class="signClass(coupon.variance)">
                {{ fmtMoney(coupon.variance) || '—' }}
              </td>
              <td>{{ couponStatusLabel(coupon) }}</td>
              <td class="row-actions">
                <RowActions
                  :receive="coupon.status === 'PENDING'"
                  @receive="startReceive(coupon)"
                  @edit="startCouponEdit(bond.id, coupon)"
                  @remove="removeCoupon(coupon)"
                />
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="card hints">
      <h3>手動步驟提醒</h3>
      <p class="muted">
        試算表的 債券 表只有現有債券 — 已到期債券由此 app 自行保存記錄。
        Overview 等尚未遷移的章節仍以試算表為準。
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
.bond-block {
  margin-top: 1rem;
}
.bond-head {
  display: flex;
  align-items: baseline;
  gap: 1rem;
}
.bond-head h3 {
  margin: 0;
}
.bond-head .muted {
  font-size: 0.8rem;
}
.unreceived {
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
.inline-form label.checkbox {
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
