<script setup lang="ts">
import type { Stock } from '../api'
import { fmtPrice } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  stocks: Stock[]
}>()

const emit = defineEmits<{
  edit: [Stock]
  remove: [Stock]
}>()
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>股票代碼</th>
        <th>Stock</th>
        <th>交易所</th>
        <th>類別</th>
        <th class="num">現價</th>
        <th class="num">PE</th>
        <th class="num">EPS</th>
        <th class="num">52週高</th>
        <th class="num">52週低</th>
        <th>備註</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="stock in props.stocks" :key="stock.id">
        <td>{{ stock.code }}</td>
        <td>{{ stock.ticker ?? '' }}</td>
        <td>{{ stock.exchange ?? '' }}</td>
        <td>{{ stock.sector ?? '' }}</td>
        <td class="num">{{ fmtPrice(stock.manual_price) }}</td>
        <td class="num">{{ fmtPrice(stock.pe) }}</td>
        <td class="num">{{ fmtPrice(stock.eps) }}</td>
        <td class="num">{{ fmtPrice(stock.high52) }}</td>
        <td class="num">{{ fmtPrice(stock.low52) }}</td>
        <td class="note">{{ stock.note ?? '' }}</td>
        <td class="row-actions">
          <RowActions @edit="emit('edit', stock)" @remove="emit('remove', stock)" />
        </td>
      </tr>
      <tr v-if="props.stocks.length === 0">
        <td colspan="11" class="muted">尚未有股票</td>
      </tr>
    </tbody>
  </table>
</template>

<style scoped>
.note {
  max-width: 16rem;
  font-size: 0.8rem;
}
.row-actions {
  white-space: nowrap;
}
</style>
