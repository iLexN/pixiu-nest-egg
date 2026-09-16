<script setup lang="ts">
import { computed } from 'vue'
import type { Stock, Trade } from '../api'
import { fmtMoney, fmtPrice, fmtShares, signClass } from '../format'
import RowActions from './RowActions.vue'

const props = defineProps<{
  trades: Trade[]
  stocks: Stock[]
  order: 'asc' | 'desc'
}>()

const emit = defineEmits<{
  edit: [Trade]
  remove: [Trade]
  'toggle-order': []
}>()

const totalCost = computed(() =>
  props.trades
    .filter((trade) => trade.trade_type === 'BUY')
    .reduce((sum, trade) => sum + trade.total, 0),
)

const priceByStock = computed(
  () => new Map(props.stocks.map((stock) => [stock.id, stock.manual_price])),
)

function currentPrice(trade: Trade): number | null {
  return priceByStock.value.get(trade.stock_id) ?? null
}

function priceClassFor(trade: Trade): string {
  const current = currentPrice(trade)
  if (trade.trade_type !== 'BUY' || trade.unit_price_incl_fee === null || current === null) {
    return ''
  }
  return signClass(current - trade.unit_price_incl_fee)
}
</script>

<template>
  <table>
    <thead>
      <tr>
        <th class="sortable" @click="emit('toggle-order')">
          日期 <span aria-hidden="true">{{ props.order === 'asc' ? '▲' : '▼' }}</span>
        </th>
        <th>股票代碼</th>
        <th>類別</th>
        <th class="num">股數</th>
        <th class="num">單價</th>
        <th class="num">fee</th>
        <th class="num">buy total</th>
        <th class="num" title="buy total ÷ 股數">平均單價（含 fee）</th>
        <th class="num">現價</th>
        <th>備註</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="trade in props.trades" :key="trade.id">
        <td>{{ trade.trade_date }}</td>
        <td>{{ trade.code }}</td>
        <td>{{ trade.trade_type }}</td>
        <td class="num">{{ fmtShares(trade.shares) }}</td>
        <td class="num">{{ fmtPrice(trade.unit_price) }}</td>
        <td class="num">{{ fmtMoney(trade.fee) }}</td>
        <td class="num">{{ fmtMoney(trade.total) }}</td>
        <td class="num">{{ fmtPrice(trade.unit_price_incl_fee) }}</td>
        <td class="num" :class="priceClassFor(trade)">{{ fmtPrice(currentPrice(trade)) }}</td>
        <td class="note">{{ trade.note ?? '' }}</td>
        <td class="row-actions">
          <RowActions @edit="emit('edit', trade)" @remove="emit('remove', trade)" />
        </td>
      </tr>
      <tr v-if="props.trades.length === 0">
        <td colspan="11" class="muted">沒有符合條件的交易</td>
      </tr>
    </tbody>
    <tfoot v-if="props.trades.length > 0">
      <tr>
        <td colspan="6">{{ props.trades.length }} 筆交易</td>
        <td class="num">{{ fmtMoney(totalCost) }}</td>
        <td colspan="4" class="muted">BUY 合計</td>
      </tr>
    </tfoot>
  </table>
</template>

<style scoped>
.sortable {
  cursor: pointer;
  user-select: none;
}
.note {
  max-width: 16rem;
  font-size: 0.8rem;
}
.row-actions {
  white-space: nowrap;
}
</style>
