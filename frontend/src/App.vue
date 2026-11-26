<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { api, toApiError, type Market } from './api'
import { fmtPriceReport } from './format'
import SummaryView from './views/SummaryView.vue'
import TradesView from './views/TradesView.vue'
import StocksView from './views/StocksView.vue'
import IbkrTransfersView from './views/IbkrTransfersView.vue'
import DepositsView from './views/DepositsView.vue'
import DepositHistoryView from './views/DepositHistoryView.vue'
import DividendsView from './views/DividendsView.vue'
import MpfView from './views/MpfView.vue'
import BondsView from './views/BondsView.vue'
import AiaView from './views/AiaView.vue'
import MonthStatView from './views/MonthStatView.vue'
import OverviewView from './views/OverviewView.vue'
import YearReviewView from './views/YearReviewView.vue'
import HappyPoolView from './views/HappyPoolView.vue'
import FamilyDepositsView from './views/FamilyDepositsView.vue'
import SettingsView from './views/SettingsView.vue'

type Tab =
  | 'overview'
  | 'trades'
  | 'summary'
  | 'stocks'
  | 'dividends'
  | 'ibkrTransfers'
  | 'deposits'
  | 'depositHistory'
  | 'mpf'
  | 'bonds'
  | 'aia'
  | 'months'
  | 'yearReview'
  | 'happyPool'
  | 'family-deposits'
  | 'settings'
type Group =
  | 'overview'
  | 'stock'
  | 'deposit'
  | 'mpf'
  | 'bond'
  | 'aia'
  | 'family'
  | 'settings'

const NAV: {
  id: Group
  label: string
  tabs: { id: Tab; label: string }[]
  links?: { label: string; href: string }[]
}[] = [
  {
    id: 'overview',
    label: '總覽',
    tabs: [
      { id: 'overview', label: '總覽' },
      { id: 'months', label: '月結' },
      { id: 'yearReview', label: '年結' },
      { id: 'happyPool', label: 'HappyPool' },
    ],
    links: [{ label: 'Export', href: '/api/export/analysis' }],
  },
  {
    id: 'stock',
    label: '股票',
    tabs: [
      { id: 'summary', label: '總覽' },
      { id: 'trades', label: '交易記錄' },
      { id: 'dividends', label: '派息' },
      { id: 'stocks', label: '管理' },
      // 美股 only — filtered out of the rendered buttons for 港股.
      { id: 'ibkrTransfers', label: '轉入記錄' },
    ],
  },
  {
    id: 'deposit',
    label: '定期',
    tabs: [
      { id: 'deposits', label: '總覽' },
      { id: 'depositHistory', label: '記錄' },
    ],
    links: [
      { label: '高息馬拉松', href: 'https://www.sc.com/hk/zh/deposits/marathon-savings-account/' },
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
    id: 'family',
    label: '家人',
    tabs: [{ id: 'family-deposits', label: '定期' }],
  },
  {
    id: 'settings',
    label: '設定',
    tabs: [{ id: 'settings', label: '設定' }],
  },
]

const market = ref<Market>('HK')
const tab = ref<Tab>('overview')
const activeGroup = computed(() => NAV.find((g) => g.tabs.some((t) => t.id === tab.value))!)
const group = computed(() => activeGroup.value.id)
// The 轉入記錄 tab stays in NAV (so activeGroup resolves) but only renders
// for 美股.
const visibleTabs = computed(() =>
  activeGroup.value.tabs.filter((t) => t.id !== 'ibkrTransfers' || market.value === 'US'),
)
watch(market, (m) => {
  if (m === 'HK' && tab.value === 'ibkrTransfers') tab.value = 'summary'
})

function selectGroup(g: (typeof NAV)[number]) {
  tab.value = g.tabs[0].id
}

const fetching = ref(false)
const fetchMessage = ref('')
// Bumped on a successful fetch so the current view remounts and reloads
// with the fresh prices.
const viewKey = ref(0)

async function fetchPrices() {
  fetching.value = true
  fetchMessage.value = ''
  try {
    const report = await api.fetchPrices(market.value)
    fetchMessage.value = fmtPriceReport(report)
    viewKey.value += 1
  } catch (err) {
    fetchMessage.value = `抓取失敗：${toApiError(err).message}`
  } finally {
    fetching.value = false
  }
}
</script>

<template>
  <header>
    <h1>財富報告</h1>
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
        <template v-for="t in visibleTabs" :key="t.id">
          <button :class="{ active: tab === t.id }" @click="tab = t.id">
            {{ t.label }}
          </button>
          <button v-if="t.id === 'stocks'" :disabled="fetching" @click="fetchPrices">
            更新現價
          </button>
        </template>
        <a
          v-for="l in activeGroup.links ?? []"
          :key="l.href"
          :href="l.href"
          :target="l.href.startsWith('http') ? '_blank' : undefined"
          :rel="l.href.startsWith('http') ? 'noopener' : undefined"
          class="tab-link"
        >
          {{ l.label }}
        </a>
      </nav>
      <span v-if="fetchMessage" class="muted">{{ fetchMessage }}</span>
    </div>
  </header>

  <main :key="viewKey">
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
    <YearReviewView v-else-if="tab === 'yearReview'" />
    <HappyPoolView v-else-if="tab === 'happyPool'" />
    <FamilyDepositsView v-else-if="tab === 'family-deposits'" />
    <SettingsView v-else-if="tab === 'settings'" />
    <IbkrTransfersView v-else-if="tab === 'ibkrTransfers'" />
    <DepositHistoryView v-else />
  </main>

  <div v-if="fetching" class="fetch-overlay">抓取中…</div>
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
.tabs button,
.tabs a.tab-link {
  background: var(--surface-alt);
}
.tabs a.tab-link {
  display: inline-block;
  padding: 0.35rem 0.75rem;
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  color: inherit;
  text-decoration: none;
}
.tabs a.tab-link:hover {
  border-color: var(--accent);
}
.fetch-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(27 42 56 / 55%);
  color: #fff;
  font-size: 1.2rem;
}
</style>
