<script setup lang="ts">
import type { Deposit } from '../api'
import { fmtBank, fmtMoney, fmtPercent } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  deposits: Deposit[]
  /** Show the 狀態 column (history section). */
  showStatus?: boolean
  emptyText?: string
}>()

const emit = defineEmits<{
  edit: [Deposit]
  remove: [Deposit]
}>()

const columns = 10 + (props.showStatus ? 1 : 0)
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>end date</th>
        <th>id</th>
        <th>銀行</th>
        <th class="num">input</th>
        <th class="num">rate</th>
        <th class="num">利息</th>
        <th class="num">total</th>
        <th v-if="props.showStatus">狀態</th>
        <th>note1</th>
        <th>note2</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="deposit in props.deposits" :key="deposit.id">
        <td>{{ deposit.end_date }}</td>
        <td>{{ deposit.label ?? '' }}</td>
        <td>{{ fmtBank(deposit.bank) }}</td>
        <td class="num">{{ fmtMoney(deposit.principal) }}</td>
        <td class="num">{{ fmtPercent(deposit.rate) }}</td>
        <td class="num">{{ fmtMoney(deposit.interest) }}</td>
        <td class="num">{{ fmtMoney(deposit.total) }}</td>
        <td v-if="props.showStatus">{{ deposit.status === 'END' ? 'End' : '' }}</td>
        <td class="note">{{ deposit.note1 ?? '' }}</td>
        <td class="note">{{ deposit.note2 ?? '' }}</td>
        <td class="row-actions">
          <RowActions @edit="emit('edit', deposit)" @remove="emit('remove', deposit)" />
        </td>
      </tr>
      <tr v-if="props.deposits.length === 0">
        <td :colspan="columns" class="muted">{{ props.emptyText ?? '沒有定期記錄' }}</td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped>
.note {
  max-width: 16rem;
  font-size: 0.8rem;
  white-space: pre-line;
}
.row-actions {
  white-space: nowrap;
}
</style>
