<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import StockForm from '../components/StockForm.vue'
import StockTable from '../components/StockTable.vue'
import { api, ApiError, type Market, type Stock } from '../api'

const props = defineProps<{ market: Market }>()

const stocks = ref<Stock[]>([])
const error = ref('')
const message = ref('')
const showForm = ref(false)
const editing = ref<Stock | null>(null)

async function load() {
  error.value = ''
  try {
    stocks.value = await api.listStocks(props.market)
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function toggleForm() {
  // While editing, the 新增…股票 button switches to a fresh create form instead of closing.
  showForm.value = editing.value ? true : !showForm.value
  editing.value = null
}

function cancelForm() {
  editing.value = null
  showForm.value = false
}

function startEdit(stock: Stock) {
  editing.value = stock
  showForm.value = true
}

function onSaved(stock: Stock) {
  message.value = editing.value ? `已更新 ${stock.code}` : `已新增 ${stock.code}`
  error.value = ''
  editing.value = null
  showForm.value = false
  void load()
}

async function toggleHide(stock: Stock) {
  error.value = ''
  message.value = ''
  try {
    await api.updateStock(stock.id, { is_active: !stock.is_active })
    message.value = stock.is_active ? `已隱藏 ${stock.code}` : `已顯示 ${stock.code}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function remove(stock: Stock) {
  error.value = ''
  message.value = ''
  if (!window.confirm(`刪除 ${stock.code}？`)) return
  try {
    await api.deleteStock(stock.id)
    message.value = `已刪除 ${stock.code}`
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
watch(() => props.market, () => {
  editing.value = null
  showForm.value = false
  void load()
})
</script>

<template>
  <section>
    <button type="button" class="toggle-form" @click="toggleForm">
      {{ `新增${props.market === 'HK' ? '港股' : '美股'}股票` }}
    </button>
    <StockForm
      v-if="showForm"
      :market="props.market"
      :editing="editing"
      @saved="onSaved"
      @cancelled="cancelForm"
    />

    <div class="card">
      <p v-if="message" class="ok">{{ message }}</p>
      <p v-if="error" class="error">{{ error }}</p>

      <StockTable :stocks="stocks" @hide="toggleHide" @edit="startEdit" @remove="remove" />
    </div>
  </section>
</template>

<style scoped>
.toggle-form {
  margin-bottom: 1rem;
}
</style>
