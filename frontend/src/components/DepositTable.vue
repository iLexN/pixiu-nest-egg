<script setup lang="ts">
import type { Deposit } from '../api'
import { fmtBank, fmtBankName, fmtMoney, fmtPercent, googleCalendarUrl } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  deposits: Deposit[]
  /** Show the 狀態 column (history section). */
  showStatus?: boolean
  /** YYYY-MM-DD; an unreceived deposit at/past it is 已到期未收. */
  today?: string
  /** Show a 加到日曆 action in the row menu. */
  calendar?: boolean
  /** Google calendar ID the 加到日曆 link pre-selects (`src`); default calendar while unset. */
  calendarSrc?: string | null
  emptyText?: string
}>()

const emit = defineEmits<{
  edit: [Deposit]
  remove: [Deposit]
  receive: [Deposit]
}>()

const columns = 10 + (props.showStatus ? 1 : 0)

function overdue(deposit: Deposit): boolean {
  return deposit.received_at === null && !!props.today && deposit.end_date <= props.today
}

function openCalendar(deposit: Deposit) {
  const parts = [deposit.label ?? '', fmtBankName(deposit.bank)].filter(Boolean).join(' - ')
  const title = parts ? `定期: ${parts}` : '定期'
  const url = googleCalendarUrl({
    title,
    date: deposit.end_date,
    lines: [
      ['到期日', deposit.end_date],
      ['編號', deposit.label ?? ''],
      ['銀行', fmtBank(deposit.bank)],
      ['本金', fmtMoney(deposit.principal)],
      ['利率', fmtPercent(deposit.rate)],
      ['利息', fmtMoney(deposit.interest)],
      ['合計', fmtMoney(deposit.total)],
      ['開始日', deposit.start_date ?? ''],
    ],
    src: props.calendarSrc,
  })
  window.open(url, '_blank', 'noopener')
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
        <td class="num" :title="deposit.rate_derived ? '由利息及日期計算' : undefined">
          {{ fmtPercent(deposit.rate) }}
        </td>
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
          <RowActions
            :removable="!deposit.received_at"
            :calendar="props.calendar"
            @calendar="openCalendar(deposit)"
            @edit="emit('edit', deposit)"
            @remove="emit('remove', deposit)"
          />
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
