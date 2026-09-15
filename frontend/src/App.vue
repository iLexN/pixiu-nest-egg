<script setup lang="ts">
import { ref } from 'vue'
import type { Market } from './api'
import SummaryView from './views/SummaryView.vue'
import TradesView from './views/TradesView.vue'
import StocksView from './views/StocksView.vue'

type Tab = 'trades' | 'summary' | 'stocks'

const market = ref<Market>('HK')
const tab = ref<Tab>('trades')
</script>

<template>
  <header>
    <h1>股票交易記錄</h1>
    <nav class="markets">
      <button :class="{ active: market === 'HK' }" @click="market = 'HK'">港股</button>
      <button :class="{ active: market === 'US' }" @click="market = 'US'">美股</button>
    </nav>
    <nav class="tabs">
      <button :class="{ active: tab === 'trades' }" @click="tab = 'trades'">交易記錄</button>
      <button :class="{ active: tab === 'summary' }" @click="tab = 'summary'">持倉總覽</button>
      <button :class="{ active: tab === 'stocks' }" @click="tab = 'stocks'">股票管理</button>
    </nav>
  </header>

  <main>
    <TradesView v-if="tab === 'trades'" :market="market" />
    <SummaryView v-else-if="tab === 'summary'" :market="market" />
    <StocksView v-else :market="market" />
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
  background: #2b6cb0;
  color: #fff;
  border-color: #2b6cb0;
}
.markets button {
  min-width: 4.5rem;
}
</style>
