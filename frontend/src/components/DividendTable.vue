<script setup lang="ts">
import { computed } from 'vue'
import type { Dividend } from '../api'
import { fmtMoney, fmtPercent, fmtPrice, fmtShares } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  dividends: Dividend[]
  /** Show a 收訖 action and the 預期派息 column on pending rows. */
  receivable?: boolean
  emptyText?: string
}>()

const emit = defineEmits<{
  receive: [Dividend]
  edit: [Dividend]
  remove: [Dividend]
}>()

const columns = computed(() => (props.receivable ? 9 : 11))
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>派息日</th>
        <th>股票</th>
        <th class="num">股數</th>
        <th class="num">總買入成本</th>
        <th class="num">每股</th>
        <th v-if="props.receivable" class="num">預期派息</th>
        <th class="num">rate</th>
        <template v-if="!props.receivable">
          <th class="num">實收派息</th>
          <th class="num">現價</th>
          <th class="num">rate</th>
        </template>
        <th>note</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="dividend in props.dividends" :key="dividend.id">
        <td>{{ dividend.pay_date }}</td>
        <td>{{ dividend.code }}</td>
        <td class="num">{{ fmtShares(dividend.shares_held) }}</td>
        <td class="num">{{ fmtMoney(dividend.buy_cost) }}</td>
        <td class="num">{{ fmtPrice(dividend.per_share) }}</td>
        <td v-if="props.receivable" class="num">{{ fmtMoney(dividend.estimated_amount) }}</td>
        <td class="num">{{ fmtPercent(dividend.yield_on_cost) }}</td>
        <template v-if="!props.receivable">
          <td class="num">{{ fmtMoney(dividend.received_amount) }}</td>
          <td class="num">{{ fmtPrice(dividend.received_price) }}</td>
          <td class="num">{{ fmtPercent(dividend.yield_on_price) }}</td>
        </template>
        <td class="note">{{ dividend.note ?? '' }}</td>
        <td class="row-actions">
          <RowActions
            :receive="props.receivable === true && dividend.status === 'PENDING'"
            @receive="emit('receive', dividend)"
            @edit="emit('edit', dividend)"
            @remove="emit('remove', dividend)"
          />
        </td>
      </tr>
      <tr v-if="props.dividends.length === 0">
        <td :colspan="columns" class="muted">{{ props.emptyText ?? '沒有派息記錄' }}</td>
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
