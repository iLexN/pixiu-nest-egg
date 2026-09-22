<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { api, ApiError, type Dividend } from '../api'
import { fmtMoney } from '../format'

const props = defineProps<{
  dividend: Dividend
}>()

const emit = defineEmits<{ saved: [Dividend]; cancelled: [] }>()

interface FormState {
  received_amount: string
  received_price: string
  also_update_price: boolean
  bank_in: boolean
}

const form = reactive<FormState>({
  received_amount: '',
  received_price: '',
  also_update_price: false,
  bank_in: true,
})
const error = ref<ApiError | null>(null)
const saving = ref(false)

watch(
  () => props.dividend,
  (dividend) => {
    form.received_amount =
      dividend.estimated_amount === null ? '' : String(dividend.estimated_amount)
    form.received_price = ''
    form.also_update_price = false
    form.bank_in = true
  },
  { immediate: true },
)

// v-model on type="number" casts to number, so accept both.
function num(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

async function submit() {
  error.value = null
  saving.value = true
  try {
    const receivedPrice = num(form.received_price)
    const saved = await api.updateDividend(props.dividend.id, {
      received_amount: num(form.received_amount),
      received_price: receivedPrice,
      bank_in: form.bank_in,
    })
    // Opt-in: the receipt price is a snapshot on this record; updating the
    // stock's 現價 is a separate, possibly different-day decision.
    if (form.also_update_price && receivedPrice !== null) {
      await api.updateStock(props.dividend.stock_id, { manual_price: receivedPrice })
    }
    emit('saved', saved)
  } catch (err) {
    error.value =
      err instanceof ApiError ? err : new ApiError(0, (err as Error).message ?? 'request failed', [])
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form class="card receive-form" @submit.prevent="submit">
    <h3>收訖派息 {{ props.dividend.code }}（派息日 {{ props.dividend.pay_date }}）</h3>

    <div class="grid">
      <label>
        實收派息
        <input v-model="form.received_amount" type="number" step="any" inputmode="decimal" />
        <small v-if="error?.fieldMessage('received_amount')" class="error">{{
          error.fieldMessage('received_amount')
        }}</small>
      </label>

      <label>
        當時現價
        <input v-model="form.received_price" type="number" step="any" inputmode="decimal" />
        <small v-if="error?.fieldMessage('received_price')" class="error">{{
          error.fieldMessage('received_price')
        }}</small>
      </label>
    </div>

    <label class="checkbox">
      <input v-model="form.bank_in" type="checkbox" />
      {{ props.dividend.market === 'US' ? '存入 IBKR USD cash' : '存入活期 HS' }}
    </label>

    <label class="checkbox">
      <input v-model="form.also_update_price" type="checkbox" />
      同時更新 {{ props.dividend.code }} 現價
    </label>

    <p v-if="props.dividend.estimated_amount !== null" class="muted">
      預期派息 {{ fmtMoney(props.dividend.estimated_amount) }}
    </p>

    <p v-if="error && error.fields.length === 0" class="error">{{ error.message }}</p>

    <div class="actions">
      <button type="submit" :disabled="saving">確認收訖</button>
      <button type="button" class="secondary" @click="emit('cancelled')">取消</button>
    </div>
  </form>
</template>

<style scoped>
.receive-form h3 {
  margin: 0 0 0.75rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
}
label {
  display: flex;
  flex-direction: column;
  font-size: 0.85rem;
  gap: 0.25rem;
}
.checkbox {
  flex-direction: row;
  align-items: center;
  margin-top: 0.75rem;
}
.actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
.muted {
  margin: 0.5rem 0 0;
}
</style>
