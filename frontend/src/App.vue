<script setup lang="ts">
import { ref } from 'vue'
import type { Market } from './api'
import SummaryView from './views/SummaryView.vue'
import TradesView from './views/TradesView.vue'
import StocksView from './views/StocksView.vue'
import DepositsView from './views/DepositsView.vue'
import DepositHistoryView from './views/DepositHistoryView.vue'

type Tab = 'trades' | 'summary' | 'stocks' | 'deposits' | 'depositHistory'

const market = ref<Market>('HK')
const tab = ref<Tab>('trades')
const STOCK_TABS: Tab[] = ['trades', 'summary', 'stocks']
</script>

<template>
  <header>
    <h1>財富記錄</h1>
    <nav v-if="STOCK_TABS.includes(tab)" class="markets">
      <button :class="{ active: market === 'HK' }" @click="market = 'HK'">港股</button>
      <button :class="{ active: market === 'US' }" @click="market = 'US'">美股</button>
    </nav>
    <nav class="tabs">
      <button :class="{ active: tab === 'trades' }" @click="tab = 'trades'">交易記錄</button>
      <button :class="{ active: tab === 'summary' }" @click="tab = 'summary'">持倉總覽</button>
      <button :class="{ active: tab === 'stocks' }" @click="tab = 'stocks'">股票管理</button>
      <button :class="{ active: tab === 'deposits' }" @click="tab = 'deposits'">定期</button>
      <button :class="{ active: tab === 'depositHistory' }" @click="tab = 'depositHistory'">
        定期記錄
      </button>
    </nav>
  </header>

  <main>
    <TradesView v-if="tab === 'trades'" :market="market" />
    <SummaryView v-else-if="tab === 'summary'" :market="market" />
    <StocksView v-else-if="tab === 'stocks'" :market="market" />
    <DepositsView v-else-if="tab === 'deposits'" />
    <DepositHistoryView v-else />
  </main>
</template>

<style scoped>
header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}
h1 {
  margin: 0;
  font-size: 1.3rem;
}
nav {
  display: flex;
  gap: 0.25rem;
}
nav button.active {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.markets button {
  min-width: 4.5rem;
}
</style>
