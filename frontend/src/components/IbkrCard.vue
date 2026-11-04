<script setup lang="ts">
import { computed, ref } from 'vue'
import { api, ApiError, type CashAccount, type IbkrBlock } from '../api'
import { fmtMoney, fmtPercent, signClass, todayIso } from '../format'

const props = defineProps<{ ibkr: IbkrBlock }>()
const emit = defineEmits<{ saved: [IbkrBlock] }>()

// The 美股 sheet's A1:B5 block — three editable cash/total inputs via PATCH
// /api/ibkr; the derived figures (computed total, net vs transferred) come
// back in the response. 轉入 lives on its own form below, through
// POST /api/ibkr/transfers.
const editing = ref(false)
const error = ref('')
const draft = ref({
  now_value: '' as number | '',
  hkd_cash: '' as number | '',
  usd_cash: '' as number | '',
})

function startEdit() {
  draft.value = {
    now_value: props.ibkr.now_value ?? '',
    hkd_cash: props.ibkr.hkd_cash ?? '',
    usd_cash: props.ibkr.usd_cash ?? '',
  }
  editing.value = true
}

function field(raw: number | '', label: string): number | null {
  if (String(raw).trim() === '') return null
  const parsed = Number(raw)
  if (!Number.isFinite(parsed) || parsed < 0) {
    throw new Error(`${label}必須是非負數字`)
  }
  return parsed
}

async function save() {
  try {
    const updated = await api.updateIbkr({
      now_value: field(draft.value.now_value, 'IBKR App 現值'),
      hkd_cash: field(draft.value.hkd_cash, 'HKD 現金'),
      usd_cash: field(draft.value.usd_cash, 'USD 現金'),
    })
    editing.value = false
    error.value = ''
    emit('saved', updated)
  } catch (err) {
    error.value = err instanceof ApiError || err instanceof Error ? err.message : String(err)
  }
}

// ----- 轉入 (bank→IBKR cash movement) -----

const transferring = ref(false)
const bankAccounts = ref<CashAccount[]>([])
const transferDraft = ref({
  amount_hkd: '' as number | '',
  transfer_date: todayIso(),
  from_account_id: 0,
})

/** The transfer source select offers HKD bank accounts only — the backend
 * enforces the same rule. */
const hkdBanks = computed(() =>
  bankAccounts.value.filter((a) => a.role === 'bank' && a.currency === 'HKD'),
)
const selectedSource = computed(() =>
  hkdBanks.value.find((a) => a.id === transferDraft.value.from_account_id),
)
const transferAmount = computed(() => {
  const raw = transferDraft.value.amount_hkd
  if (raw === '') return null
  const amount = Number(raw)
  return Number.isFinite(amount) ? amount : null
})

async function startTransfer() {
  try {
    // Fresh balances so the before→after preview is current.
    bankAccounts.value = await api.listCashAccounts()
    transferDraft.value = {
      amount_hkd: '',
      transfer_date: todayIso(),
      from_account_id:
        hkdBanks.value.find((a) => a.label === 'HS')?.id ?? hkdBanks.value[0]?.id ?? 0,
    }
    transferring.value = true
    error.value = ''
  } catch (err) {
    error.value = err instanceof ApiError || err instanceof Error ? err.message : String(err)
  }
}

/** Refetch the block; `saved` lets the parent reload (總覽) or swap the
 * block (美股 → 總覽). */
async function refresh() {
  emit('saved', await api.ibkr())
}

async function saveTransfer() {
  const amount = transferAmount.value
  if (amount === null || amount <= 0) {
    error.value = '轉入金額必須是正數'
    return
  }
  try {
    await api.createIbkrTransfer({
      amount_hkd: amount,
      transfer_date: transferDraft.value.transfer_date || undefined,
      from_account_id: transferDraft.value.from_account_id || undefined,
    })
    transferring.value = false
    error.value = ''
    await refresh()
  } catch (err) {
    error.value = err instanceof ApiError || err instanceof Error ? err.message : String(err)
  }
}
</script>

<template>
  <section class="card">
    <h3>IBKR <small class="muted">A/C U20178201</small></h3>
    <form v-if="editing" class="ibkr-form" @submit.prevent="save">
      <label>
        IBKR App 現值 (HKD)
        <input v-model="draft.now_value" type="number" step="any" inputmode="decimal" />
      </label>
      <label>
        HKD 現金
        <input v-model="draft.hkd_cash" type="number" step="any" inputmode="decimal" />
      </label>
      <label>
        USD 現金
        <input v-model="draft.usd_cash" type="number" step="any" inputmode="decimal" />
      </label>
      <div class="ibkr-actions">
        <button type="submit">儲存</button>
        <button type="button" class="link" @click="editing = false">取消</button>
      </div>
    </form>
    <template v-else>
      <table>
        <tbody>
          <tr>
            <td>累計轉入 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.transferred_hkd) || '—' }}</td>
            <td>IBKR App 現值 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.now_value) || '—' }}</td>
          </tr>
          <tr>
            <td>HKD 現金</td>
            <td class="num">{{ fmtMoney(ibkr.hkd_cash) || '—' }}</td>
            <td>USD 現金</td>
            <td class="num">{{ fmtMoney(ibkr.usd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>美股總市值 (USD)</td>
            <td class="num">{{ fmtMoney(ibkr.stock_value_usd) || '—' }}</td>
            <td>計算總值 (HKD)</td>
            <td class="num">{{ fmtMoney(ibkr.computed_total_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>淨額 / 回報率</td>
            <td class="num" :class="signClass(ibkr.net)">
              {{ fmtMoney(ibkr.net) || '—' }} / {{ fmtPercent(ibkr.net_pct) || '—' }}
            </td>
            <td>計算 − App 差異</td>
            <td class="num" :class="signClass(ibkr.vs_now_value)">
              {{ fmtMoney(ibkr.vs_now_value) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
      <div class="ibkr-actions">
        <button type="button" class="link" @click="startEdit">編輯</button>
        <button type="button" class="link" @click="startTransfer">轉入</button>
      </div>
      <form v-if="transferring" class="ibkr-form" @submit.prevent="saveTransfer">
        <label>
          轉入 (HKD)
          <input
            v-model="transferDraft.amount_hkd"
            type="number"
            step="any"
            inputmode="decimal"
            placeholder="+金額"
          />
        </label>
        <label>
          轉入日期
          <input v-model="transferDraft.transfer_date" type="date" :max="todayIso()" />
        </label>
        <label>
          轉出帳戶
          <select v-model.number="transferDraft.from_account_id">
            <option v-for="account in hkdBanks" :key="account.id" :value="account.id">
              {{ account.label }}（{{ fmtMoney(account.amount) }}）
            </option>
          </select>
        </label>
        <div class="ibkr-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="transferring = false">取消</button>
        </div>
        <p v-if="selectedSource && transferAmount !== null" class="muted ibkr-preview">
          {{ selectedSource.label }} {{ fmtMoney(selectedSource.amount) }} →
          {{ fmtMoney(selectedSource.amount - transferAmount) }}；IBKR HKD
          {{ fmtMoney(ibkr.hkd_cash ?? 0) }} →
          {{ fmtMoney((ibkr.hkd_cash ?? 0) + transferAmount) }}
        </p>
      </form>
    </template>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.ibkr-form {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
}
.ibkr-form label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.2rem;
}
.ibkr-form input {
  width: 9rem;
}
.ibkr-actions {
  display: flex;
  gap: 0.5rem;
}
.ibkr-preview {
  flex-basis: 100%;
  margin: 0;
}
</style>
