<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import DepositForm from '../components/DepositForm.vue'
import DepositTable from '../components/DepositTable.vue'
import {
  api,
  ApiError,
  type Deposit,
  type DepositSummary,
  type ManualAsset,
} from '../api'
import { fmtBank, fmtMoney, todayIso } from '../format'

const summary = ref<DepositSummary | null>(null)
const assets = ref<ManualAsset[]>([])
const message = ref('')
const error = ref('')
const editing = ref<Deposit | null>(null)

const cashAssets = computed(() => assets.value.filter((asset) => asset.kind === 'cash'))

/** Bank code → the cash manual_asset label it deposits into. */
const BANK_ASSET_LABELS: Record<string, string> = { SC: '渣打', HS: 'HS' }

function numOrNull(raw: string | number): number | null {
  const parsed = Number(raw)
  return String(raw).trim() === '' || !Number.isFinite(parsed) ? null : parsed
}

async function load() {
  error.value = ''
  try {
    const [summaryData, assetsData] = await Promise.all([
      api.depositSummary(),
      api.listManualAssets(),
    ])
    summary.value = summaryData
    assets.value = assetsData
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function remove(deposit: Deposit) {
  const name = deposit.label ?? `#${deposit.id}`
  if (!window.confirm(`刪除定期 ${name}（${deposit.end_date} 到期）？`)) return
  try {
    await api.deleteDeposit(deposit.id)
    message.value = `已刪除定期 ${name}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function onSaved(deposit: Deposit) {
  message.value = `已儲存定期 #${deposit.id}`
  error.value = ''
  editing.value = null
  void load()
}

// ----- 收訖 (matured -> received, optional bank-in) -----

const receiving = ref<{
  deposit: Deposit
  received_at: string
  interest: string | number
  credit_asset_id: number | null
  credit_amount: string | number
} | null>(null)

function startReceive(deposit: Deposit) {
  const assetLabel = deposit.bank ? (BANK_ASSET_LABELS[deposit.bank] ?? deposit.bank) : ''
  const matched = cashAssets.value.find((asset) => asset.label === assetLabel)
  receiving.value = {
    deposit,
    received_at: todayIso(),
    interest: deposit.interest ?? '',
    credit_asset_id: matched?.id ?? null,
    credit_amount: deposit.total,
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
  const creditAmount = numOrNull(current.credit_amount)
  if (current.credit_asset_id !== null && creditAmount === null) {
    error.value = '存入金額必須是數字'
    return
  }
  try {
    await api.receiveDeposit(current.deposit.id, {
      received_at: current.received_at || undefined,
      interest: interest ?? undefined,
      credit_asset_id: current.credit_asset_id ?? undefined,
      credit_amount: creditAmount ?? undefined,
    })
    receiving.value = null
    message.value = '已記錄收訖'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <section>
    <DepositForm
      v-if="editing"
      :editing="editing"
      @saved="onSaved"
      @cancelled="editing = null"
    />

    <div v-if="summary" class="card">
      <h3>未到期定期</h3>
      <div class="totals-grid" aria-label="定期合計">
        <div class="total-card">
          <span>Total（本金）</span>
          <strong>{{ fmtMoney(summary.active_totals.principal) }}</strong>
        </div>
        <div class="total-card">
          <span>利息</span>
          <strong>{{ fmtMoney(summary.active_totals.interest) }}</strong>
        </div>
        <div class="total-card">
          <span>到期收回</span>
          <strong>{{ fmtMoney(summary.active_totals.total) }}</strong>
        </div>
      </div>

      <DepositTable
        :deposits="summary.upcoming"
        :today="summary.today"
        empty-text="沒有未到期的定期"
        @edit="editing = $event"
        @remove="remove"
        @receive="startReceive"
      />

      <form v-if="receiving" class="inline-form" @submit.prevent="saveReceive">
        <h4>
          收訖 — {{ receiving.deposit.label ?? `#${receiving.deposit.id}` }}（{{
            receiving.deposit.end_date
          }}
          到期）
        </h4>
        <label>
          收訖日
          <input v-model="receiving.received_at" type="date" required />
        </label>
        <label>
          實收利息
          <input v-model="receiving.interest" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          存入活期
          <select v-model="receiving.credit_asset_id">
            <option :value="null">不存入</option>
            <option v-for="asset in cashAssets" :key="asset.id" :value="asset.id">
              {{ asset.label }}（{{ fmtMoney(asset.amount) }}）
            </option>
          </select>
        </label>
        <label v-if="receiving.credit_asset_id !== null">
          存入金額
          <input
            v-model="receiving.credit_amount"
            type="number"
            step="any"
            inputmode="decimal"
          />
        </label>
        <div class="form-actions">
          <button type="submit">收訖</button>
          <button type="button" class="link" @click="receiving = null">取消</button>
        </div>
      </form>

      <h4>到期月份</h4>
      <table class="rollup">
        <thead>
          <tr>
            <th>月份</th>
            <th class="num">Total</th>
            <th class="num">利息</th>
            <th class="num">定期</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="bucket in summary.months" :key="`${bucket.year}-${bucket.month}`">
            <td>{{ bucket.year }} 年 {{ bucket.month }} 月</td>
            <td class="num">{{ fmtMoney(bucket.total) }}</td>
            <td class="num">{{ fmtMoney(bucket.interest) }}</td>
            <td class="num">{{ fmtMoney(bucket.principal) }}</td>
          </tr>
          <tr v-if="summary.months.length === 0">
            <td colspan="4" class="muted">沒有未到期的定期</td>
          </tr>
        </tbody>
      </table>

      <h4>銀行分佈</h4>
      <table class="rollup">
        <thead>
          <tr>
            <th>銀行</th>
            <th class="num">Total</th>
            <th class="num">利息</th>
            <th class="num">定期</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="bank in summary.banks" :key="bank.bank">
            <td>{{ fmtBank(bank.bank) }}</td>
            <td class="num">{{ fmtMoney(bank.total) }}</td>
            <td class="num">{{ fmtMoney(bank.interest) }}</td>
            <td class="num">{{ fmtMoney(bank.principal) }}</td>
          </tr>
          <tr v-if="summary.banks.length === 0">
            <td colspan="4" class="muted">沒有未到期的定期</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="card hints">
      <h3>手動步驟提醒</h3>
      <div class="hint-columns">
        <div>
          <h4>定期 start step</h4>
          <ol>
            <li>月結 - 調整</li>
            <li>add row 回報率</li>
            <li>add row 定期 ref</li>
            <li>update overview - 預測</li>
          </ol>
        </div>
        <div>
          <h4>定期 end step</h4>
          <ol>
            <li>定期 - 收訖（自動：月結調整 + 利息 + 存入活期）</li>
            <li>money master</li>
            <li>remove - 定期 row</li>
          </ol>
        </div>
      </div>
      <p class="muted">
        收訖後定期才離開未到期清單並計入月結利息；money master、回報率、Overview 預測
        仍是試算表手動步驟。
      </p>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<style scoped>
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.75rem;
  margin: 1rem 0;
}
.total-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.75rem;
}
.total-card span {
  color: var(--muted);
  display: block;
  font-size: 0.78rem;
  margin-bottom: 0.25rem;
}
.total-card strong {
  font-size: 1.05rem;
  font-weight: 650;
}
h4 {
  margin: 1.5rem 0 0.5rem;
}
.rollup {
  table-layout: fixed;
}
.rollup th.num {
  width: 9rem;
}
.hint-columns {
  display: flex;
  flex-wrap: wrap;
  gap: 2rem;
}
.hint-columns h4 {
  margin: 0 0 0.4rem;
}
.hint-columns ol {
  margin: 0;
  padding-left: 1.25rem;
}
</style>
