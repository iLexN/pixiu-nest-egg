<script setup lang="ts">
import { onMounted, ref } from 'vue'
import DepositForm from '../components/DepositForm.vue'
import DepositTable from '../components/DepositTable.vue'
import { api, ApiError, type Deposit } from '../api'

const showForm = ref(false)
const editing = ref<Deposit | null>(null)

const history = ref<Deposit[]>([])
const historyYears = ref<number[]>([])
const selectedYear = ref<number>(new Date().getFullYear())
const message = ref('')
const error = ref('')

async function load() {
  error.value = ''
  try {
    const summary = await api.depositSummary()
    historyYears.value = summary.history_years
    if (!historyYears.value.includes(selectedYear.value)) {
      selectedYear.value = historyYears.value.at(-1) ?? selectedYear.value
    }
    await loadHistory()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function loadHistory() {
  editing.value = null
  try {
    history.value = await api.listDeposits({ year: selectedYear.value, order: 'desc' })
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

async function unreceive(deposit: Deposit) {
  const name = deposit.label ?? `#${deposit.id}`
  if (!window.confirm(`取消收訖 ${name}？已存入的活期與月結調整項目會一併還原。`)) return
  try {
    await api.unreceiveDeposit(deposit.id)
    message.value = `已取消收訖 ${name}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
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
  error.value = ''
  editing.value = null
  showForm.value = false
  void load()
}

onMounted(load)
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
        @unreceive="unreceive"
      />
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
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
