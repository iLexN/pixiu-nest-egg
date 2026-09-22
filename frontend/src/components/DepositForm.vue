<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { api, ApiError, type Deposit } from '../api'
import DateInput from './DateInput.vue'
import { fmtMoney, todayIso } from '../format'

const props = defineProps<{
  editing: Deposit | null
}>()

const emit = defineEmits<{ saved: [Deposit]; cancelled: [] }>()

interface FormState {
  label: string
  bank: string
  principal: string
  rate: string
  interest: string
  start_date: string
  end_date: string
  note1: string
  note2: string
}

function emptyForm(): FormState {
  return {
    label: '',
    bank: '',
    principal: '',
    rate: '',
    interest: '',
    start_date: todayIso(),
    end_date: todayIso(),
    note1: '',
    note2: '',
  }
}

const form = reactive<FormState>(emptyForm())
const error = ref<ApiError | null>(null)
const saving = ref(false)

watch(
  () => props.editing,
  (deposit) => {
    if (!deposit) {
      Object.assign(form, emptyForm())
      return
    }
    Object.assign(form, {
      label: deposit.label ?? '',
      bank: deposit.bank ?? '',
      principal: deposit.principal === null ? '' : String(deposit.principal),
      // Stored as a fraction; edited as a percent.
      rate: deposit.rate === null ? '' : String(deposit.rate * 100),
      interest: deposit.interest === null ? '' : String(deposit.interest),
      start_date: deposit.start_date ?? '',
      end_date: deposit.end_date,
      note1: deposit.note1 ?? '',
      note2: deposit.note2 ?? '',
    })
  },
  { immediate: true },
)

function num(value: string | number | null): number | null {
  const text = String(value ?? '').trim()
  if (text === '') return null
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : null
}

const preview = computed(() => {
  const principal = num(form.principal)
  const interest = num(form.interest)
  if (principal === null && interest === null) return null
  return { total: (principal ?? 0) + (interest ?? 0) }
})

async function submit() {
  error.value = null
  saving.value = true
  try {
    // The form takes rate as a percent (2.8); the API stores the fraction.
    const ratePct = num(form.rate)
    const payload = {
      label: String(form.label).trim() === '' ? null : String(form.label).trim(),
      bank: String(form.bank).trim() === '' ? null : String(form.bank).trim(),
      principal: num(form.principal),
      rate: ratePct === null ? null : ratePct / 100,
      interest: num(form.interest),
      start_date: form.start_date === '' ? null : form.start_date,
      end_date: form.end_date,
      note1: String(form.note1).trim() === '' ? null : String(form.note1).trim(),
      note2: String(form.note2).trim() === '' ? null : String(form.note2).trim(),
    }
    const saved = props.editing
      ? await api.updateDeposit(props.editing.id, payload)
      : await api.createDeposit(payload)
    if (!props.editing) Object.assign(form, emptyForm())
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
  <form class="card deposit-form" @submit.prevent="submit">
    <h3>
      {{ props.editing ? `編輯定期 ${props.editing.label ?? `#${props.editing.id}`}` : '新增定期' }}
    </h3>

    <div class="grid">
      <label>
        id
        <input v-model="form.label" type="text" placeholder="SC-9632" />
        <small v-if="error?.fieldMessage('label')" class="error">{{
          error.fieldMessage('label')
        }}</small>
      </label>

      <label>
        銀行
        <input v-model="form.bank" type="text" list="bank-codes" placeholder="SC" />
        <datalist id="bank-codes">
          <option value="SC">渣打</option>
          <option value="HS">恒生</option>
        </datalist>
      </label>

      <label>
        input（本金）
        <input v-model="form.principal" type="number" step="any" inputmode="decimal" />
        <small v-if="error?.fieldMessage('principal')" class="error">{{
          error.fieldMessage('principal')
        }}</small>
      </label>

      <label>
        rate（%）
        <input v-model="form.rate" type="number" step="any" inputmode="decimal" placeholder="2.8" />
        <small v-if="error?.fieldMessage('rate')" class="error">{{
          error.fieldMessage('rate')
        }}</small>
      </label>

      <label>
        利息
        <input v-model="form.interest" type="number" step="any" inputmode="decimal" />
        <small v-if="error?.fieldMessage('interest')" class="error">{{
          error.fieldMessage('interest')
        }}</small>
      </label>

      <label>
        開始日
        <DateInput v-model="form.start_date" />
        <small v-if="error?.fieldMessage('start_date')" class="error">{{
          error.fieldMessage('start_date')
        }}</small>
      </label>

      <label>
        end date
        <DateInput v-model="form.end_date" required />
        <small v-if="error?.fieldMessage('end_date')" class="error">{{
          error.fieldMessage('end_date')
        }}</small>
      </label>

      <label>
        note1
        <input v-model="form.note1" type="text" />
      </label>

      <label class="wide">
        note2
        <input v-model="form.note2" type="text" placeholder="分段利率等備註" />
      </label>
    </div>

    <p v-if="preview" class="preview">
      <span>total（本金 + 利息） <strong>{{ fmtMoney(preview.total) }}</strong></span>
    </p>
    <p v-else class="preview muted">填入 input 或 利息 後顯示 total</p>

    <p v-if="error && error.fields.length === 0" class="error">{{ error.message }}</p>

    <div class="actions">
      <button type="submit" :disabled="saving">{{ props.editing ? '儲存' : '新增' }}</button>
      <button v-if="props.editing" type="button" class="secondary" @click="emit('cancelled')">
        取消
      </button>
    </div>
  </form>
</template>

<style scoped>
.deposit-form h3 {
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
.wide {
  grid-column: 1 / -1;
}
.preview {
  display: flex;
  flex-wrap: wrap;
  gap: 1.5rem;
  margin: 0.75rem 0 0;
  font-size: 0.9rem;
}
.actions {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
</style>
