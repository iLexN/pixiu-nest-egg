<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, ApiError, type IbkrTransfer } from '../api'
import { fmtMoney } from '../format'

// 股票 → 轉入記錄 (美股 only): the ibkr_transfers log with per-row delete —
// a delete reverses the recorded cash movement exactly.
const transfers = ref<IbkrTransfer[]>([])
const message = ref('')
const error = ref('')

const total = computed(() => transfers.value.reduce((sum, t) => sum + t.amount_hkd, 0))

async function load() {
  error.value = ''
  try {
    transfers.value = await api.listIbkrTransfers()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

async function remove(transfer: IbkrTransfer) {
  const label = transfer.from_account_label ?? ''
  if (
    !window.confirm(
      `刪除 ${transfer.transfer_date} ${label}轉入 ${fmtMoney(transfer.amount_hkd)}？`,
    )
  )
    return
  try {
    await api.deleteIbkrTransfer(transfer.id)
    message.value = `已刪除 ${transfer.transfer_date} 轉入 ${fmtMoney(transfer.amount_hkd)}`
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
    <div class="card">
      <h3>IBKR 轉入記錄</h3>
      <p v-if="!transfers.length" class="muted">暫無轉入記錄</p>
      <table v-else>
        <thead>
          <tr>
            <th>日期</th>
            <th class="num">金額</th>
            <th>轉出帳戶</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in transfers" :key="t.id">
            <td>{{ t.transfer_date }}</td>
            <td class="num">{{ fmtMoney(t.amount_hkd) }}</td>
            <td>{{ t.from_account_label ?? '—' }}</td>
            <td>
              <button type="button" class="link" @click="remove(t)">刪除</button>
            </td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>累計</td>
            <td class="num">{{ fmtMoney(total) }}</td>
            <td colspan="2"></td>
          </tr>
        </tfoot>
      </table>
    </div>
    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>
