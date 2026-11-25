<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type ForecastResponse, type OverviewResponse } from '../api'
import ForecastGrid from '../components/ForecastGrid.vue'
import IbkrCard from '../components/IbkrCard.vue'
import InlineForm from '../components/InlineForm.vue'
import { fmtMoney, fmtPercent, fmtPercentShort, fmtPrice, signClass } from '../format'
import { numOrNull } from '../parse'
import { usePage } from '../page'

/** `remain ≤ 0` means the goal is already reached — show 已達成, not years. */
function reached(remain: number | null): boolean {
  return remain !== null && remain <= 0
}

function fmtYears(value: number | null): string {
  return value === null ? '' : value.toFixed(2)
}

const data = ref<OverviewResponse | null>(null)
const forecast = ref<ForecastResponse | null>(null)

const { message, error, reload, run } = usePage(async () => {
  const [overviewData, forecastData] = await Promise.all([api.overview(), api.forecast()])
  data.value = overviewData
  forecast.value = forecastData
})

// Inline amount edit for manual asset/cash rows (PATCH /api/manual-assets).
const editingId = ref<number | null>(null)
/** Which table the inline amount edit targets. */
const editingTarget = ref<'cash' | 'asset'>('asset')
const editDraft = ref<number | ''>('')

/** The grid's 轉為定期 finished — the form did the write, the page reports. */
function onForecastConverted() {
  message.value = '已轉為定期'
  reload()
}

function startEdit(id: number, current: number | null, target: 'cash' | 'asset' = 'asset') {
  editingId.value = id
  editingTarget.value = target
  editDraft.value = current ?? ''
}

function saveEdit() {
  run(async () => {
    const id = editingId.value
    if (id === null) return
    const amount = numOrNull(editDraft.value)
    if (amount === null) {
      throw new Error('金額必須是數字')
    }
    // 活期 cash rows are cash_accounts; asset rows are manual_assets.
    if (editingTarget.value === 'cash') {
      await api.updateCashAccount(id, { amount })
    } else {
      await api.updateManualAsset(id, { amount })
    }
    editingId.value = null
  }, { message: '已更新' })
}

// ----- 活期調撥 (POST /api/cash-accounts/moves) -----

const moveOpen = ref(false)
const moveFrom = ref<number | null>(null)
const moveTo = ref<number | null>(null)
const moveAmount = ref<number | ''>('')

const cashRows = computed(() => data.value?.semi_liquid.cash_rows ?? [])

function openMove() {
  const rows = cashRows.value
  moveFrom.value = rows[0]?.id ?? null
  moveTo.value = rows.length === 2 ? rows[1].id : null
  moveAmount.value = ''
  moveOpen.value = true
}

/** With exactly two bank rows, picking one side sets the other. */
function pairMove(side: 'from' | 'to') {
  const rows = cashRows.value
  if (rows.length !== 2) return
  const picked = side === 'from' ? moveFrom.value : moveTo.value
  const other = picked === null ? null : (rows.find((row) => row.id !== picked)?.id ?? null)
  if (side === 'from') moveTo.value = other
  else moveFrom.value = other
}

function saveMove() {
  run(async () => {
    const amount = numOrNull(moveAmount.value)
    if (amount === null) {
      throw new Error('金額必須是數字')
    }
    if (moveFrom.value === null || moveTo.value === null) {
      throw new Error('請選擇調出及調入戶口')
    }
    if (moveFrom.value === moveTo.value) {
      throw new Error('調出及調入戶口不可相同')
    }
    await api.bankMove({
      from_account_id: moveFrom.value,
      to_account_id: moveTo.value,
      amount,
    })
    moveOpen.value = false
  }, { message: '已調撥' })
}

// ----- 投資目標 formula footnote -----

/** The current calendar year's row; the footnote expands its target. */
const targetNoteRow = computed(() => {
  const year = new Date().getFullYear()
  return data.value?.invest_targets.rows.find((row) => row.year === year) ?? null
})

/** The six formula terms with their contributions (backend-computed);
 *  null without a current-year row. */
const targetNoteTerms = computed(() => {
  const terms = targetNoteRow.value?.terms
  if (!terms) return null
  return {
    priorInvested: terms.prior_invested,
    priorInterest: terms.prior_interest,
    priorPool: terms.prior_pool,
    raiseTerm: terms.raise_term,
    interest: terms.interest,
    pool: terms.pool,
  }
})

// ----- 目標 存/年 footnote -----

/** The five L12 yearly terms: raw monthly inputs plus their yearly
 *  contributions from `goals.saving_terms` (null while absent). */
const savingYearTerms = computed(() => {
  const d = data.value
  if (!d) return null
  const terms = d.goals.saving_terms
  return {
    saved: d.averages.saved,
    saved12: terms.saved_year,
    avgInvested: terms.avg_invested,
    mm: d.money_master.avg_per_month,
    mm12: terms.mm_year,
    savedAvg: d.goals.saved_avg,
    running12: terms.running_saved_year,
    interestAvg: d.goals.interest_avg,
    combined12: terms.running_combined_year,
  }
})

/** The 3rd/Final rows with their 基數/步伐 cells resolved: the figure each
 *  row's remain subtracts and the pace its years divide by, with live values. */
const formulaGoalRows = computed(() => {
  const d = data.value
  if (!d) return []
  return [
    {
      name: '3rd',
      goal: d.goals.third,
      base: { label: '流動資產', value: d.liquid_assets },
      pace: { label: '(歷年存+息)×12', value: d.goals.third.pace },
    },
    {
      name: 'Final',
      goal: d.goals.final_goal,
      base: { label: '總數', value: d.total_assets },
      pace: { label: '歷年總變化×12', value: d.goals.final_goal.pace },
    },
  ]
})

onMounted(reload)
</script>

<template>
  <p v-if="error" class="error">{{ error?.message }}</p>
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
          <tr v-for="row in data.assets" :key="row.key + row.label" :title="row.note ?? undefined">
            <td :class="{ 'has-note': row.note }">{{ row.label }}</td>
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
                @click="startEdit(row.id, row.amount, 'cash')"
              >
                編輯
              </button>
            </td>
          </tr>
          <tr>
            <td>活期</td>
            <td class="num">{{ fmtMoney(data.semi_liquid.cash_sum) || '—' }}</td>
            <td>
              <button
                v-if="!moveOpen && cashRows.length >= 2"
                type="button"
                class="link"
                @click="openMove"
              >
                調撥
              </button>
            </td>
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
              {{ fmtPercent(data.semi_liquid.share) || '—' }} ·
              與{{ fmtPercentShort(data.semi_liquid_target) }}流動相差
              <span :class="signClass(data.semi_liquid.vs_quarter_liquid)">
                {{ fmtMoney(data.semi_liquid.vs_quarter_liquid) || '—' }}
              </span>
            </td>
          </tr>
        </tfoot>
      </table>
      <InlineForm v-if="moveOpen" title="活期調撥" @submit="saveMove" @cancel="moveOpen = false">
        <label>
          調出
          <select v-model="moveFrom" required @change="pairMove('from')">
            <option v-for="row in cashRows" :key="row.id" :value="row.id">{{ row.label }}</option>
          </select>
        </label>
        <label>
          調入
          <select v-model="moveTo" required @change="pairMove('to')">
            <option v-for="row in cashRows" :key="row.id" :value="row.id">{{ row.label }}</option>
          </select>
        </label>
        <label>
          金額
          <input
            v-model="moveAmount"
            type="number"
            step="any"
            min="0"
            inputmode="decimal"
            required
          />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>策略</h3>
      <table>
        <tbody>
          <tr>
            <td>可動用</td>
            <td
              class="num"
              :class="{ negative: (data.liquidity_tiers.can_use ?? 0) < 0 }"
            >
              {{ fmtMoney(data.liquidity_tiers.can_use) || '—' }}
            </td>
          </tr>
          <tr>
            <td>不可動用（6個月薪金）</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.cannot_use) || '—' }}</td>
          </tr>
          <tr>
            <td>短期可取回</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.short_term) || '—' }}</td>
          </tr>
          <tr>
            <td>長期可取回</td>
            <td class="num">{{ fmtMoney(data.liquidity_tiers.long_term) || '—' }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <ForecastGrid
      v-if="forecast"
      :forecast="forecast"
      :run="run" @converted="onForecastConverted"
    />
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
      <p v-if="targetNoteTerms" class="muted note-formula">
        <span>目標 =</span>
        <span>
          <span class="note-line">上年invested ({{
            fmtMoney(targetNoteTerms.priorInvested) || '—'
          }}) −</span>
          <span class="note-line">上年利息回報 ({{
            fmtMoney(targetNoteTerms.priorInterest) || '—'
          }}) −</span>
          <span class="note-line">上年Pool支出×0.7 ({{
            fmtMoney(targetNoteTerms.priorPool) || '—'
          }}) +</span>
          <span class="note-line">月薪增幅×50% * 12 ({{
            fmtMoney(targetNoteTerms.raiseTerm)
          }}) +</span>
          <span class="note-line">今年利息回報 ({{
            fmtMoney(targetNoteTerms.interest) || '—'
          }}) +</span>
          <span class="note-line">今年Pool支出×0.7 ({{
            fmtMoney(targetNoteTerms.pool) || '—'
          }})。</span>
        </span>
      </p>
    </div>

    <div class="card goals-card">
      <h3>目標</h3>
      <table>
        <thead>
          <tr>
            <th></th>
            <th class="num">目標金額</th>
            <th class="num">基數</th>
            <th class="num">剩餘</th>
            <th class="num">步伐</th>
            <th class="num">年</th>
            <th class="num">日期</th>
          </tr>
        </thead>
        <tbody>
          <!-- 1st/2nd share the 現值 (K13/K19) base and the 存/年 (L12) pace. -->
          <tr v-for="(m, i) in data.goals.milestones" :key="i">
            <td>{{ i === 0 ? '1st' : '2nd' }}</td>
            <td class="num">{{ fmtMoney(m.target) || '—' }}</td>
            <td class="num term-cell">
              現值<span class="muted">{{ fmtMoney(data.goals.current_invest) || '—' }}</span>
            </td>
            <td class="num" :class="signClass(m.remain)">
              {{ fmtMoney(m.remain) || '—' }}
            </td>
            <td class="num term-cell">
              存/年<span class="muted">{{ fmtMoney(data.goals.saving_year) || '—' }}</span>
            </td>
            <td class="num">
              {{ reached(m.remain) ? '已達成' : fmtYears(m.years) || '—' }}
            </td>
            <td class="num">{{ reached(m.remain) ? '已達成' : m.eta || '—' }}</td>
          </tr>
          <!-- 3rd/Final: remain vs 流動資產/總數, each with its own pace. -->
          <tr v-for="row in formulaGoalRows" :key="row.name">
            <td>{{ row.name }}</td>
            <td class="num">{{ fmtMoney(row.goal.target) || '—' }}</td>
            <td class="num term-cell">
              {{ row.base.label
              }}<span class="muted">{{ fmtMoney(row.base.value) || '—' }}</span>
            </td>
            <td class="num" :class="signClass(row.goal.remain)">
              {{ fmtMoney(row.goal.remain) || '—' }}
            </td>
            <td class="num term-cell">
              {{ row.pace.label
              }}<span class="muted">{{ fmtMoney(row.pace.value) || '—' }}</span>
            </td>
            <td class="num">
              {{ reached(row.goal.remain) ? '已達成' : fmtYears(row.goal.years) || '—' }}
            </td>
            <td class="num">{{ reached(row.goal.remain) ? '已達成' : row.goal.eta || '—' }}</td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>現值</td>
            <td class="num">{{ fmtMoney(data.goals.current_invest) || '—' }}</td>
            <td colspan="5"></td>
          </tr>
          <tr>
            <td>存/年</td>
            <td class="num">{{ fmtMoney(data.goals.saving_year) || '—' }}</td>
            <td>存/月</td>
            <td class="num">{{ fmtMoney(data.goals.saving_month) || '—' }}</td>
            <td colspan="3"></td>
          </tr>
          <tr>
            <td>回報/年</td>
            <td class="num">{{ fmtMoney(data.goals.passive_return_year) || '—' }}</td>
            <td>回報/月</td>
            <td class="num">{{ fmtMoney(data.goals.passive_return_month) || '—' }}</td>
            <td colspan="3"></td>
          </tr>
        </tfoot>
      </table>
      <p class="muted">
        年均回報率 N4 = {{ fmtPercent(data.goals.n4_rate) || '—'
        }}<template v-if="data.goals.n4_year !== null"
          >（{{ data.goals.n4_year }}）</template
        >
        · 目標金額及常數於 月結 → 設定 編輯
      </p>
      <p class="muted">
        3rd = 生活預算 ({{ fmtMoney(data.averages.living_budget) || '—' }})
        × 通脹 ({{ data.goals.goal3_inflation ?? '—' }})
        × 12 × 年期 ({{ data.goals.goal3_years ?? '—' }})
      </p>
      <p class="muted">
        Final = 薪金 ({{ fmtMoney(data.salary) || '—' }})
        × 替代率 ({{ fmtPercentShort(data.goals.final_replacement) || '—' }})
        × 12 ÷ N4 ({{ fmtPercent(data.goals.n4_rate) || '—' }})
      </p>
      <p v-if="savingYearTerms" class="muted note-formula">
        <span>存/年 = AVERAGE(</span>
        <span>
          <span class="note-line">近12月存×12 ({{
            fmtMoney(savingYearTerms.saved) || '—'
          }}
            x 12 = {{ fmtMoney(savingYearTerms.saved12) || '—' }}),</span>
          <span class="note-line">近3年invested ({{
            fmtMoney(savingYearTerms.avgInvested) || '—'
          }}),</span>
          <span class="note-line">MM月均×12 ({{ fmtMoney(savingYearTerms.mm) || '—' }}
            x 12 = {{ fmtMoney(savingYearTerms.mm12) || '—' }}),</span>
          <span class="note-line">歷年存×12 ({{
            fmtMoney(savingYearTerms.savedAvg) || '—'
          }}
            x 12 = {{ fmtMoney(savingYearTerms.running12) || '—' }}),</span>
          <span class="note-line">(歷年存+息)×12 (({{
            fmtMoney(savingYearTerms.savedAvg) || '—'
          }}
            + {{ fmtMoney(savingYearTerms.interestAvg) || '—' }}) x 12 =
            {{ fmtMoney(savingYearTerms.combined12) || '—' }}))</span>
        </span>
      </p>
    </div>

    <div class="card money-master">
      <h3>Money Master</h3>
      <table>
        <thead>
          <tr>
            <th></th>
            <th class="num">月</th>
            <th class="num">已儲</th>
            <th class="num">平均</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>現在</td>
            <td class="num">{{ data.money_master.month_now ?? '—' }}</td>
            <td class="num">{{ fmtMoney(data.money_master.saved) || '—' }}</td>
            <td class="num">
              {{ fmtMoney(data.money_master.avg_per_month) || '—'
              }}{{ data.money_master.avg_per_month != null ? '/mo' : '' }}
            </td>
          </tr>
          <tr>
            <td>目標</td>
            <td class="num">{{ data.money_master.target_months ?? '—' }}</td>
            <td class="num">{{ fmtMoney(data.money_master.target_amount) || '—' }}</td>
            <td class="num">
              {{ fmtMoney(data.money_master.yearly_rate) || '—'
              }}{{ data.money_master.yearly_rate != null ? '/yr' : '' }}
            </td>
          </tr>
          <tr>
            <td>進度</td>
            <td class="num">{{ fmtPercent(data.money_master.time_progress) || '—' }}</td>
            <td class="num">{{ fmtPercent(data.money_master.saved_progress) || '—' }}</td>
            <td class="num" :class="signClass(data.money_master.progress_gap)">
              {{ fmtPercent(data.money_master.progress_gap) || '—' }}
            </td>
          </tr>
        </tbody>
        <tfoot>
          <tr>
            <td>每月需儲</td>
            <td class="num" :class="signClass(data.money_master.coming_save)">
              {{ fmtMoney(data.money_master.coming_save) || '—' }}
            </td>
            <td colspan="2"></td>
          </tr>
          <tr>
            <td>可動用</td>
            <td class="num" :class="signClass(data.money_master.can_use)">
              {{ fmtMoney(data.money_master.can_use) || '—' }}
            </td>
            <td colspan="2"></td>
          </tr>
        </tfoot>
      </table>
      <p class="muted">於 總覽 → 月結 → 設定 更新 Money Master 數字</p>
    </div>

    <IbkrCard class="ibkr-card" :ibkr="data.ibkr" @saved="reload" />
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
.overview-grid .card.invest-targets,
.overview-grid .card.goals-card,
.overview-grid .card.ibkr-card {
  grid-column: span 2;
}
/* A strong rule separates the derived avg pace from the challenge inputs. */
.money-master td:nth-child(4),
.money-master th:nth-child(4) {
  border-left: 3px double var(--border-strong);
}
/* Same strong separator between the inputs (現在/目標) and the 進度 row. */
.money-master tbody tr:nth-child(3) td {
  border-top: 3px double var(--border-strong);
}
.overview-grid tfoot td.muted {
  white-space: normal;
}
/* A manual asset row with a note gets a hover cue: the note rides the row's
   native title tooltip. */
.has-note {
  cursor: help;
  text-decoration: underline dotted;
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
/* 基數/步伐 cells: the term name on one line, its live value muted beneath. */
.term-cell .muted {
  display: block;
  font-size: 0.8rem;
  white-space: nowrap;
}
.note-formula {
  display: flex;
  gap: 0.4ch;
}
.note-line {
  display: block;
}
h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
}
</style>
