<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, ApiError, type MonthSummary, type MonthYearSummary } from '../api'
import { fmtMoney } from '../format'

const summary = ref<MonthSummary | null>(null)
const error = ref('')
const copiedYear = ref<number | null>(null)

/** Tab-separated plain numbers so a paste into Google Sheets fills adjacent cells. */
async function copyRow(year: MonthYearSummary) {
  const cells = [
    year.pool_balance,
    year.pool_income,
    year.pool_input_sum,
    year.entertainment_sum,
  ].map((v) => v.toFixed(2))
  try {
    await navigator.clipboard.writeText([String(year.year), ...cells].join('\t'))
    copiedYear.value = year.year
    setTimeout(() => {
      if (copiedYear.value === year.year) copiedYear.value = null
    }, 1500)
  } catch (err) {
    error.value = `複製失敗: ${err instanceof Error ? err.message : String(err)}`
  }
}

/** Years with any pool activity, oldest first — all-zero years stay hidden. */
const years = computed(() =>
  (summary.value?.years ?? []).filter(
    (year) =>
      year.pool_income !== 0 || year.pool_input_sum !== 0 || year.entertainment_sum !== 0,
  ),
)

async function load() {
  error.value = ''
  try {
    summary.value = await api.monthSummary()
  } catch (err) {
    summary.value = null
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <p v-if="error" class="error">{{ error }}</p>

  <template v-if="summary">
    <div class="card">
      <h3>HappyPool</h3>
      <div class="totals-grid featured">
        <div class="total-card">
          <span>開心 Pool</span>
          <strong>{{ fmtMoney(summary.pool_balance) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>Irene payable</span>
          <strong>{{ fmtMoney(summary.pool_totals.irene_payable) || '—' }}</strong>
        </div>
      </div>
      <div class="totals-grid">
        <div class="total-card">
          <span>iLex</span>
          <strong>{{ fmtMoney(summary.pool_totals.ilex_input) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>Irene</span>
          <strong>{{ fmtMoney(summary.pool_totals.irene_input) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>平均支出</span>
          <strong>{{ fmtMoney(summary.pool_totals.avg_spend) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>平均 input</span>
          <strong>{{ fmtMoney(summary.pool_totals.avg_input) || '—' }}</strong>
        </div>
      </div>
    </div>

    <div class="card">
      <table>
        <thead>
          <tr>
            <th>年</th>
            <th class="num">結餘</th>
            <th class="num">iLex input</th>
            <th class="num">Irene input</th>
            <th class="num">支出</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="year in years" :key="year.year">
            <td>{{ year.year }}</td>
            <td class="num">{{ fmtMoney(year.pool_balance) || '—' }}</td>
            <td class="num">{{ fmtMoney(year.pool_income) || '—' }}</td>
            <td class="num">{{ fmtMoney(year.pool_input_sum) || '—' }}</td>
            <td class="num">{{ fmtMoney(year.entertainment_sum) || '—' }}</td>
            <td>
              <button type="button" class="link" @click="copyRow(year)">
                {{ copiedYear === year.year ? 'copied' : 'copy' }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </template>
</template>

<style scoped>
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 0.5rem;
}
.totals-grid + .totals-grid {
  margin-top: 0.5rem;
}
.featured {
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}
.featured .total-card {
  border-color: var(--accent);
  border-left: 4px solid var(--accent);
  background: var(--card);
  padding: 0.75rem 1rem;
}
.featured .total-card > strong {
  font-size: 1.5rem;
  color: var(--accent);
}
.total-card {
  background: var(--surface-alt);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;
}
.total-card > span {
  display: block;
  font-size: 0.8rem;
  color: var(--muted);
}
.total-card > strong {
  font-size: 1.05rem;
  font-variant-numeric: tabular-nums;
}
h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
}
</style>
