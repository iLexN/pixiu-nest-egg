<script setup lang="ts">
import { computed, ref } from 'vue'
import { api, ApiError, type IbkrBlock } from '../api'
import { fmtMoney, fmtPercent, signClass, todayIso } from '../format'

const props = defineProps<{ ibkr: IbkrBlock }>()
const emit = defineEmits<{ saved: [IbkrBlock] }>()

// The 美股 sheet's A1:B5 block — four manual inputs; the derived figures
// (computed total, net vs transferred) come back in the response.
const editing = ref(false)
const error = ref('')
const draft = ref({
  transferred_delta: '' as number | '',
  transfer_date: '',
  now_value: '' as number | '',
  hkd_cash: '' as number | '',
  usd_cash: '' as number | '',
})

// The 轉入 input is a delta; this previews the new cumulative total.
const transferredPreview = computed(() => {
  const text = String(draft.value.transferred_delta).trim()
  if (text === '') return null
  const delta = Number(text)
  if (!Number.isFinite(delta)) return null
  return (props.ibkr.transferred_hkd ?? 0) + delta
})

function startEdit() {
  draft.value = {
    transferred_delta: '',
    transfer_date: todayIso(),
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
    // Undefined records nothing; a delta appends a dated transfer-log row.
    const deltaText = String(draft.value.transferred_delta).trim()
    const delta = deltaText === '' ? undefined : Number(deltaText)
    if (delta !== undefined && !Number.isFinite(delta)) {
      throw new Error('轉入金額必須是數字')
    }
    const updated = await api.updateIbkr({
      transfer_hkd: delta,
      transfer_date: draft.value.transfer_date || undefined,
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
</script>

<template>
  <section class="card">
    <h3>IBKR <small class="muted">A/C U20178201</small></h3>
    <form v-if="editing" class="ibkr-form" @submit.prevent="save">
      <label>
        轉入 (HKD)
        <input
          v-model="draft.transferred_delta"
          type="number"
          step="any"
          inputmode="decimal"
          placeholder="+金額"
        />
      </label>
      <label>
        轉入日期
        <input v-model="draft.transfer_date" type="date" />
      </label>
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
      <p class="muted ibkr-preview">
        累計轉入 {{ fmtMoney(ibkr.transferred_hkd) || '0' }}
        <template v-if="transferredPreview !== null">
          → <strong>{{ fmtMoney(transferredPreview) }}</strong>
        </template>
      </p>
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
      <button type="button" class="link" @click="startEdit">編輯</button>
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
