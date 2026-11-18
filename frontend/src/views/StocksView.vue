<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import StockForm from '../components/StockForm.vue'
import StockTable from '../components/StockTable.vue'
import { api, type Market, type Stock } from '../api'
import { usePage } from '../page'

const props = defineProps<{ market: Market }>()

const { data, message, error, reload, run } = usePage(() => api.listStocks(props.market))
const stocks = computed(() => data.value ?? [])
const showForm = ref(false)
const editing = ref<Stock | null>(null)

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
  editing.value = null
  showForm.value = false
  reload()
}

function toggleHide(stock: Stock) {
  run(() => api.updateStock(stock.id, { is_active: !stock.is_active }), {
    message: stock.is_active ? `已隱藏 ${stock.code}` : `已顯示 ${stock.code}`,
  })
}

function remove(stock: Stock) {
  run(() => api.deleteStock(stock.id), {
    confirm: `刪除 ${stock.code}？`,
    message: `已刪除 ${stock.code}`,
  })
}

onMounted(reload)
watch(() => props.market, () => {
  editing.value = null
  showForm.value = false
  reload()
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
      <p v-if="error" class="error">{{ error?.message }}</p>

      <StockTable :stocks="stocks" @hide="toggleHide" @edit="startEdit" @remove="remove" />
    </div>
  </section>
</template>

<style scoped>
.toggle-form {
  margin-bottom: 1rem;
}
</style>
