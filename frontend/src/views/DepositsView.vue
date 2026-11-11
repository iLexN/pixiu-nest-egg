<script setup lang="ts">
import { onMounted, ref } from 'vue'
import DepositForm from '../components/DepositForm.vue'
import DepositReceiveForm from '../components/DepositReceiveForm.vue'
import DepositTable from '../components/DepositTable.vue'
import {
  api,
  ApiError,
  type CashAccount,
  type Deposit,
  type DepositSummary,
} from '../api'
import { fmtBank, fmtMoney } from '../format'

const summary = ref<DepositSummary | null>(null)
const accounts = ref<CashAccount[]>([])
const privateCalendarId = ref<string | null>(null)
const message = ref('')
const error = ref('')
const editing = ref<Deposit | null>(null)

async function load() {
  error.value = ''
  try {
    const [summaryData, accountsData, settingsData] = await Promise.all([
      api.depositSummary(),
      api.listCashAccounts(),
      api.getSettings(),
    ])
    summary.value = summaryData
    accounts.value = accountsData
    privateCalendarId.value = settingsData.calendar_private_id
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

const receiving = ref<Deposit | null>(null)

function onReceived() {
  receiving.value = null
  message.value = '已記錄收訖'
  error.value = ''
  void load()
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

    <DepositReceiveForm
      v-if="receiving"
      :key="receiving.id"
      class="card"
      :deposit="receiving"
      :accounts="accounts"
      @saved="onReceived"
      @cancelled="receiving = null"
    />

    <div v-if="summary" class="card">
      <h3>未到期定期</h3>
      <div class="totals-grid" aria-label="定期合計">
        <div class="total-card">
          <span>本金</span>
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
        calendar
        :calendar-src="privateCalendarId"
        empty-text="沒有未到期的定期"
        @edit="editing = $event"
        @remove="remove"
        @receive="receiving = $event"
      />

      <h4>到期月份</h4>
      <table class="rollup">
        <thead>
          <tr>
            <th>月份</th>
            <th class="num">合計</th>
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
            <th class="num">合計</th>
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
            <li>新增定期 / 轉為定期（自動：扣活期 + 月結調整 −本金）</li>
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
        收訖後定期才離開未到期清單並計入月結利息；money master 在月結設定更新，回報率、Overview
        預測 仍是試算表手動步驟。
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
