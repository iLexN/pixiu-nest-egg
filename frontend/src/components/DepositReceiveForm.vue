<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, ApiError, type CashAccount, type Deposit } from '../api'
import { fmtMoney, todayIso } from '../format'

const props = defineProps<{
  deposit: Deposit
  accounts: CashAccount[]
}>()

const emit = defineEmits<{ saved: [Deposit]; cancelled: [] }>()

/** HKD cash accounts a deposit can bank into. */
const cashAccounts = computed(() =>
  props.accounts.filter((account) => account.currency === 'HKD'),
)

/** Bank code → the cash account label it deposits into. */
const BANK_ASSET_LABELS: Record<string, string> = { SC: '渣打', HS: 'HS' }

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

/** The deposit's own bank row wins; otherwise the currency's default receipt
 * account, else 不存入. */
function defaultAccount(): number | null {
  const accountLabel = props.deposit.bank
    ? (BANK_ASSET_LABELS[props.deposit.bank] ?? props.deposit.bank)
    : ''
  const matched = cashAccounts.value.find((account) => account.label === accountLabel)
  const fallback = cashAccounts.value.find((account) => account.is_default_receipt)
  return (matched ?? fallback)?.id ?? null
}

const received_at = ref(todayIso())
const interest = ref<string | number>(props.deposit.interest ?? '')
const account_id = ref<number | null>(defaultAccount())
const error = ref('')
const formEl = ref<HTMLFormElement | null>(null)

onMounted(() => formEl.value?.scrollIntoView({ behavior: 'smooth', block: 'start' }))

/** What the cash row will be credited: principal + 實收利息 (display only). */
const receivePreview = computed(
  () =>
    (props.deposit.principal ?? 0) + (numOrNull(interest.value) ?? props.deposit.interest ?? 0),
)

async function save() {
  const parsed = numOrNull(interest.value)
  if (interest.value !== '' && parsed === null) {
    error.value = '實收利息必須是數字'
    return
  }
  try {
    const deposit = await api.receiveDeposit(props.deposit.id, {
      received_at: received_at.value || undefined,
      interest: parsed ?? undefined,
      ...(account_id.value === null ? { bank_in: false } : { account_id: account_id.value }),
    })
    emit('saved', deposit)
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}
</script>

<template>
  <form ref="formEl" class="inline-form" @submit.prevent="save">
    <h4>收訖 — {{ deposit.label ?? `#${deposit.id}` }}（{{ deposit.end_date }} 到期）</h4>
    <label>
      收訖日
      <input v-model="received_at" type="date" required />
    </label>
    <label>
      實收利息
      <input v-model="interest" type="number" step="any" inputmode="decimal" />
    </label>
    <label>
      存入活期
      <select v-model="account_id">
        <option :value="null">不存入</option>
        <option v-for="account in cashAccounts" :key="account.id" :value="account.id">
          {{ account.label }}（{{ fmtMoney(account.amount) }}）
        </option>
      </select>
    </label>
    <p v-if="account_id !== null" class="muted">
      存入 {{ fmtMoney(receivePreview) }}（本金 + 實收利息）
    </p>
    <p v-if="error" class="error">{{ error }}</p>
    <div class="form-actions">
      <button type="submit">收訖</button>
      <button type="button" class="link" @click="emit('cancelled')">取消</button>
    </div>
  </form>
</template>

<style scoped>
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
.error {
  grid-column: 1 / -1;
  margin: 0;
}
</style>
