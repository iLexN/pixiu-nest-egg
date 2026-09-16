<script setup lang="ts">
import { onMounted, ref } from 'vue'
import DepositForm from '../components/DepositForm.vue'
import DepositTable from '../components/DepositTable.vue'
import { api, ApiError, type Deposit, type DepositSummary } from '../api'
import { fmtBank, fmtMoney } from '../format'

const summary = ref<DepositSummary | null>(null)
const message = ref('')
const error = ref('')
const editing = ref<Deposit | null>(null)

async function load() {
  error.value = ''
  try {
    summary.value = await api.depositSummary()
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
        empty-text="沒有未到期的定期"
        @edit="editing = $event"
        @remove="remove"
      />

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
            <li>month stat - 調整</li>
            <li>add row 回報率</li>
            <li>add row 定期 ref</li>
            <li>update overview - 預測</li>
          </ol>
        </div>
        <div>
          <h4>定期 end step</h4>
          <ol>
            <li>month stat - 調整</li>
            <li>month stat - 利息</li>
            <li>money master</li>
            <li>remove - 定期 row</li>
          </ol>
        </div>
      </div>
      <p class="muted">這些仍是試算表手動步驟；Month Stat、回報率、Overview 尚未遷移。</p>
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
