<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, ApiError, type OverviewResponse } from '../api'
import { fmtMoney, fmtPercent, fmtPrice, signClass } from '../format'

const data = ref<OverviewResponse | null>(null)
const message = ref('')
const error = ref('')

// Inline amount edit for manual asset/cash rows (PATCH /api/manual-assets).
const editingId = ref<number | null>(null)
const editDraft = ref<number | ''>('')

async function load() {
  try {
    data.value = await api.overview()
    error.value = ''
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

function startEdit(id: number, current: number | null) {
  editingId.value = id
  editDraft.value = current ?? ''
}

async function saveEdit() {
  const id = editingId.value
  if (id === null) return
  const amount = Number(editDraft.value)
  if (String(editDraft.value).trim() === '' || !Number.isFinite(amount)) {
    error.value = '金額必須是數字'
    return
  }
  try {
    await api.updateManualAsset(id, { amount })
    editingId.value = null
    message.value = '已更新'
    error.value = ''
    await load()
  } catch (err) {
    error.value = err instanceof ApiError ? err.message : String(err)
  }
}

onMounted(load)
</script>

<template>
  <p v-if="error" class="error">{{ error }}</p>
  <p v-if="message" class="ok">{{ message }}</p>

  <template v-if="data">
    <div class="card">
      <div class="totals-grid">
        <div class="total-card">
          <span>總數</span>
          <strong>{{ fmtMoney(data.total_assets) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>流動資產</span>
          <strong>{{ fmtMoney(data.liquid_assets) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>流動資產 ÷ 薪金×100</span>
          <strong>{{ fmtPercent(data.liquid_ratio) || '—' }}</strong>
        </div>
        <div class="total-card">
          <span>開心 Pool</span>
          <strong>{{ fmtMoney(data.averages.pool_balance) || '—' }}</strong>
        </div>
      </div>
      <p class="muted">
        USD→HKD {{ fmtPrice(data.rate) || '—' }} · 薪金
        {{ fmtMoney(data.salary) || '—' }}
      </p>
    </div>

    <div class="overview-grid">
    <div class="card">
      <h3>資產</h3>
      <table>
        <thead>
          <tr>
            <th>項目</th>
            <th class="num">金額</th>
            <th class="num">佔比</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in data.assets" :key="row.key + row.label">
            <td>{{ row.label }}</td>
            <td
              v-if="row.manual_asset_id !== null && editingId === row.manual_asset_id"
              class="num"
            >
              <input
                v-model="editDraft"
                type="number"
                step="any"
                inputmode="decimal"
                class="cell-input"
                @keyup.enter="saveEdit"
                @keyup.esc="editingId = null"
              />
            </td>
            <td v-else class="num">{{ fmtMoney(row.amount) || '—' }}</td>
            <td class="num">{{ fmtPercent(row.share) || '—' }}</td>
            <td>
              <template v-if="row.manual_asset_id !== null">
                <template v-if="editingId === row.manual_asset_id">
                  <button type="button" class="link" @click="saveEdit">儲存</button>
                  <button type="button" class="link" @click="editingId = null">取消</button>
                </template>
                <button
                  v-else
                  type="button"
                  class="link"
                  @click="startEdit(row.manual_asset_id, row.amount)"
                >
                  編輯
                </button>
              </template>
            </td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>合計</td>
            <td class="num">{{ fmtMoney(data.assets_sum) || '—' }}</td>
            <td colspan="2"></td>
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="card">
      <h3>過去 12 個月平均</h3>
      <table>
        <tbody>
          <tr>
            <td>總數增加</td>
            <td class="num">{{ fmtMoney(data.averages.total_change) || '—' }}</td>
          </tr>
          <tr>
            <td>支出</td>
            <td class="num">{{ fmtMoney(data.averages.month_spend) || '—' }}</td>
          </tr>
          <tr>
            <td>生活支出</td>
            <td class="num">{{ fmtMoney(data.averages.living_spend) || '—' }}</td>
          </tr>
          <tr>
            <td>生活預算</td>
            <td
              class="num"
              :class="
                data.averages.living_budget === null
                  ? ''
                  : signClass(
                      data.averages.living_budget_floor - data.averages.living_budget,
                    )
              "
              :title="`低於 ${fmtMoney(data.averages.living_budget_floor)} 為綠，高於轉紅`"
            >
              {{ fmtMoney(data.averages.living_budget) || '—' }}
            </td>
          </tr>
          <tr>
            <td>存</td>
            <td class="num">{{ fmtMoney(data.averages.saved) || '—' }}</td>
          </tr>
          <tr>
            <td>利息</td>
            <td class="num">{{ fmtMoney(data.averages.interest) || '—' }}</td>
          </tr>
        </tbody>
      </table>
      <p v-if="data.averages.window_start" class="muted">
        {{ data.averages.window_start.slice(0, 7) }} ~
        {{ data.averages.window_end?.slice(0, 7) }}
      </p>
    </div>

    <div class="card">
      <h3>半流動資金</h3>
      <table>
        <tbody>
          <tr>
            <td>已定期</td>
            <td class="num">{{ fmtMoney(data.semi_liquid.deposits) || '—' }}</td>
            <td></td>
          </tr>
          <tr v-for="row in data.semi_liquid.cash_rows" :key="row.id">
            <td>{{ row.label }}</td>
            <td v-if="editingId === row.id" class="num">
              <input
                v-model="editDraft"
                type="number"
                step="any"
                inputmode="decimal"
                class="cell-input"
                @keyup.enter="saveEdit"
                @keyup.esc="editingId = null"
              />
            </td>
            <td v-else class="num">{{ fmtMoney(row.amount) || '—' }}</td>
            <td>
              <template v-if="editingId === row.id">
                <button type="button" class="link" @click="saveEdit">儲存</button>
                <button type="button" class="link" @click="editingId = null">取消</button>
              </template>
              <button
                v-else
                type="button"
                class="link"
                @click="startEdit(row.id, row.amount)"
              >
                編輯
              </button>
            </td>
          </tr>
          <tr>
            <td>活期</td>
            <td class="num">{{ fmtMoney(data.semi_liquid.cash_sum) || '—' }}</td>
            <td></td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>半流動資金</td>
            <td class="num" :class="signClass(data.semi_liquid.vs_quarter_liquid)">
              {{ fmtMoney(data.semi_liquid.total) || '—' }}
            </td>
            <td></td>
          </tr>
          <tr>
            <td colspan="3" class="num muted">
              {{ fmtPercent(data.semi_liquid.share) || '—' }} · 與25%流動相差
              <span :class="signClass(data.semi_liquid.vs_quarter_liquid)">
                {{ fmtMoney(data.semi_liquid.vs_quarter_liquid) || '—' }}
              </span>
            </td>
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="card">
      <h3>IBKR</h3>
      <table>
        <tbody>
          <tr>
            <td>累計轉入 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.transferred_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>IBKR App 現值 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.now_value) || '—' }}</td>
          </tr>
          <tr>
            <td>HKD 現金</td>
            <td class="num">{{ fmtMoney(data.ibkr.hkd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>USD 現金</td>
            <td class="num">{{ fmtMoney(data.ibkr.usd_cash) || '—' }}</td>
          </tr>
          <tr>
            <td>美股總市值 (USD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.stock_value_usd) || '—' }}</td>
          </tr>
          <tr>
            <td>計算總值 (HKD)</td>
            <td class="num">{{ fmtMoney(data.ibkr.computed_total_hkd) || '—' }}</td>
          </tr>
          <tr>
            <td>淨額 / 回報率</td>
            <td class="num" :class="signClass(data.ibkr.net)">
              {{ fmtMoney(data.ibkr.net) || '—' }} /
              {{ fmtPercent(data.ibkr.net_pct) || '—' }}
            </td>
          </tr>
          <tr>
            <td>計算 − App 差異</td>
            <td class="num" :class="signClass(data.ibkr.vs_now_value)">
              {{ fmtMoney(data.ibkr.vs_now_value) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
      <p class="muted">於 股票 → 美股 → 總覽 編輯 IBKR 數字</p>
    </div>

    <div class="card invest-targets">
      <h3>投資目標</h3>
      <p class="muted">
        近3年平均 invested
        <strong class="avg">{{ fmtMoney(data.invest_targets.avg_invested) || '—' }}</strong>
      </p>
      <table>
        <thead>
          <tr>
            <th>年份</th>
            <th class="num">invested</th>
            <th class="num">目標</th>
            <th class="num">剩餘</th>
            <th class="num">增長</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in data.invest_targets.rows" :key="row.year">
            <td>{{ row.year }}</td>
            <td class="num">{{ fmtMoney(row.invested) || '—' }}</td>
            <td class="num">{{ fmtMoney(row.target) || '—' }}</td>
            <td class="num" :class="signClass(row.remain)">
              {{ fmtMoney(row.remain) || '—' }}
            </td>
            <td class="num" :class="signClass(row.growth)">
              {{ fmtPercent(row.growth) || '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    </div>
  </template>
</template>

<style scoped>
.overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 1rem;
  margin-bottom: 1rem;
}
.overview-grid .card {
  margin-bottom: 0;
  min-width: 0;
  overflow-x: auto;
}
.overview-grid .card.invest-targets {
  grid-column: span 2;
}
.overview-grid tfoot td.muted {
  white-space: normal;
}
.totals-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 0.5rem;
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
.cell-input {
  width: 8rem;
  text-align: right;
}
.avg {
  color: var(--text);
}
h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
}
</style>
