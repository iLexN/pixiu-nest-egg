<script setup lang="ts">
import type { Deposit } from '../api'
import { fmtBank, fmtMoney, fmtPercent } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  deposits: Deposit[]
  /** Show the 狀態 column (history section). */
  showStatus?: boolean
  /** YYYY-MM-DD; an unreceived deposit at/past it is 已到期未收. */
  today?: string
  emptyText?: string
}>()

const emit = defineEmits<{
  edit: [Deposit]
  remove: [Deposit]
  receive: [Deposit]
  unreceive: [Deposit]
}>()

const columns = 10 + (props.showStatus ? 1 : 0)

function overdue(deposit: Deposit): boolean {
  return deposit.received_at === null && !!props.today && deposit.end_date <= props.today
}
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>到期日</th>
        <th>編號</th>
        <th>銀行</th>
        <th class="num">本金</th>
        <th class="num">利率</th>
        <th class="num">利息</th>
        <th class="num">合計</th>
        <th v-if="props.showStatus">狀態</th>
        <th>備註1</th>
        <th>備註2</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="deposit in props.deposits" :key="deposit.id">
        <td>
          {{ deposit.end_date }}
          <small v-if="overdue(deposit)" class="overdue">已到期未收</small>
        </td>
        <td>{{ deposit.label ?? '' }}</td>
        <td>{{ fmtBank(deposit.bank) }}</td>
        <td class="num">{{ fmtMoney(deposit.principal) }}</td>
        <td class="num">{{ fmtPercent(deposit.rate) }}</td>
        <td class="num">{{ fmtMoney(deposit.interest) }}</td>
        <td class="num">{{ fmtMoney(deposit.total) }}</td>
        <td v-if="props.showStatus">
          {{ deposit.received_at ? `收訖 ${deposit.received_at}` : '未收' }}
        </td>
        <td class="note">{{ deposit.note1 ?? '' }}</td>
        <td class="note">{{ deposit.note2 ?? '' }}</td>
        <td class="row-actions">
          <button
            v-if="!deposit.received_at"
            type="button"
            class="link"
            @click="emit('receive', deposit)"
          >
            收訖
          </button>
          <button
            v-else
            type="button"
            class="link"
            @click="emit('unreceive', deposit)"
          >
            取消收訖
          </button>
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
.overdue {
  color: var(--negative, #b91c1c);
}
</style>
