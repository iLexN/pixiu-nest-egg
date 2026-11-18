<script setup lang="ts">
import { onMounted, ref } from 'vue'
import DepositForm from '../components/DepositForm.vue'
import DepositReceiveForm from '../components/DepositReceiveForm.vue'
import DepositTable from '../components/DepositTable.vue'
import { api, type CashAccount, type Deposit } from '../api'
import { usePage } from '../page'

const showForm = ref(false)
const editing = ref<Deposit | null>(null)
const receiving = ref<Deposit | null>(null)
const accounts = ref<CashAccount[]>([])

const history = ref<Deposit[]>([])
const historyYears = ref<number[]>([])
const selectedYear = ref<number>(new Date().getFullYear())

async function fetchHistory() {
  editing.value = null
  history.value = await api.listDeposits({ year: selectedYear.value, order: 'desc' })
}

const { message, error, reload, run } = usePage(async () => {
  const [summary, accountsData] = await Promise.all([
    api.depositSummary(),
    api.listCashAccounts(),
  ])
  historyYears.value = summary.history_years
  accounts.value = accountsData
  if (!historyYears.value.includes(selectedYear.value)) {
    selectedYear.value = historyYears.value.at(-1) ?? selectedYear.value
  }
  await fetchHistory()
})

function loadHistory() {
  run(fetchHistory, { reload: false })
}

function remove(deposit: Deposit) {
  const name = deposit.label ?? `#${deposit.id}`
  run(() => api.deleteDeposit(deposit.id), {
    confirm: `刪除定期 ${name}（${deposit.end_date} 到期）？`,
    message: `已刪除定期 ${name}`,
  })
}

function toggleForm() {
  // While editing, 新增定期 switches to a fresh create form instead of closing.
  showForm.value = editing.value ? true : !showForm.value
  editing.value = null
}

function cancelForm() {
  editing.value = null
  showForm.value = false
}

function startEdit(deposit: Deposit) {
  editing.value = deposit
  showForm.value = true
}

function onSaved(deposit: Deposit) {
  message.value = `已儲存定期 #${deposit.id}`
  editing.value = null
  showForm.value = false
  reload()
}

function onReceived() {
  message.value = '已記錄收訖'
  receiving.value = null
  reload()
}

onMounted(reload)
</script>

<template>
  <section>
    <button type="button" class="toggle-form" @click="toggleForm">新增定期</button>
    <DepositForm
      v-if="showForm"
      :editing="editing"
      @saved="onSaved"
      @cancelled="cancelForm"
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

    <div class="card">
      <div class="history-header">
        <h3>定期記錄</h3>
        <label>
          年份
          <select v-model="selectedYear" @change="loadHistory">
            <option v-for="year in historyYears" :key="year" :value="year">
              {{ year }}
            </option>
          </select>
        </label>
      </div>

      <DepositTable
        :deposits="history"
        :show-status="true"
        empty-text="這年沒有到期記錄"
        @edit="startEdit"
        @remove="remove"
        @receive="receiving = $event"
      />
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error?.message }}</p>
  </section>
</template>

<style scoped>
.toggle-form {
  margin-bottom: 1rem;
}
.history-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}
.history-header h3 {
  margin: 0;
}
.history-header label {
  display: flex;
  align-items: center;
  font-size: 0.85rem;
  gap: 0.4rem;
}
</style>
