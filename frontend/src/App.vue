<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Market } from './api'
import SummaryView from './views/SummaryView.vue'
import TradesView from './views/TradesView.vue'
import StocksView from './views/StocksView.vue'
import DepositsView from './views/DepositsView.vue'
import DepositHistoryView from './views/DepositHistoryView.vue'
import DividendsView from './views/DividendsView.vue'
import MpfView from './views/MpfView.vue'
import BondsView from './views/BondsView.vue'
import AiaView from './views/AiaView.vue'
import MonthStatView from './views/MonthStatView.vue'
import OverviewView from './views/OverviewView.vue'

type Tab =
  | 'overview'
  | 'trades'
  | 'summary'
  | 'stocks'
  | 'dividends'
  | 'deposits'
  | 'depositHistory'
  | 'mpf'
  | 'bonds'
  | 'aia'
  | 'months'
type Group = 'overview' | 'stock' | 'deposit' | 'mpf' | 'bond' | 'aia' | 'months'

const NAV: { id: Group; label: string; tabs: { id: Tab; label: string }[] }[] = [
  {
    id: 'overview',
    label: '總覽',
    tabs: [{ id: 'overview', label: '總覽' }],
  },
  {
    id: 'stock',
    label: '股票',
    tabs: [
      { id: 'summary', label: '總覽' },
      { id: 'trades', label: '交易記錄' },
      { id: 'dividends', label: '派息' },
      { id: 'stocks', label: '管理' },
    ],
  },
  {
    id: 'deposit',
    label: '定期',
    tabs: [
      { id: 'deposits', label: '總覽' },
      { id: 'depositHistory', label: '記錄' },
    ],
  },
  {
    id: 'mpf',
    label: 'MPF',
    tabs: [{ id: 'mpf', label: '總覽' }],
  },
  {
    id: 'bond',
    label: '債券',
    tabs: [{ id: 'bonds', label: '總覽' }],
  },
  {
    id: 'aia',
    label: 'AIA',
    tabs: [{ id: 'aia', label: '總覽' }],
  },
  {
    id: 'months',
    label: '月結',
    tabs: [{ id: 'months', label: '總覽' }],
  },
]

const market = ref<Market>('HK')
const tab = ref<Tab>('summary')
const activeGroup = computed(() => NAV.find((g) => g.tabs.some((t) => t.id === tab.value))!)
const group = computed(() => activeGroup.value.id)

function selectGroup(g: (typeof NAV)[number]) {
  tab.value = g.tabs[0].id
}
</script>

<template>
  <header>
    <h1>財富記錄</h1>
    <nav class="groups">
      <button
        v-for="g in NAV"
        :key="g.id"
        :class="{ active: group === g.id }"
        @click="selectGroup(g)"
      >
        {{ g.label }}
      </button>
    </nav>
    <div class="subnav">
      <nav v-if="group === 'stock'" class="markets">
        <button :class="{ active: market === 'HK' }" @click="market = 'HK'">港股</button>
        <button :class="{ active: market === 'US' }" @click="market = 'US'">美股</button>
      </nav>
      <nav class="tabs">
        <button
          v-for="t in activeGroup.tabs"
          :key="t.id"
          :class="{ active: tab === t.id }"
          @click="tab = t.id"
        >
          {{ t.label }}
        </button>
      </nav>
    </div>
  </header>

  <main>
    <OverviewView v-if="tab === 'overview'" />
    <TradesView v-else-if="tab === 'trades'" :market="market" />
    <SummaryView v-else-if="tab === 'summary'" :market="market" />
    <StocksView v-else-if="tab === 'stocks'" :market="market" />
    <DividendsView v-else-if="tab === 'dividends'" :market="market" />
    <DepositsView v-else-if="tab === 'deposits'" />
    <MpfView v-else-if="tab === 'mpf'" />
    <BondsView v-else-if="tab === 'bonds'" />
    <AiaView v-else-if="tab === 'aia'" />
    <MonthStatView v-else-if="tab === 'months'" />
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
.subnav {
  display: flex;
  flex-basis: 100%;
  align-items: center;
  gap: 1rem;
}
.tabs button {
  background: var(--surface-alt);
}
</style>
