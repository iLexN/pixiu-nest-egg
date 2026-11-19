<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import {
  api,
  type AppSettings,
  type CashAccount,
  type CashRole,
  type Currency,
  type ManualAsset,
  type ManualAssetLiquidity,
  type NewCashAccount,
  type NewManualAsset,
  type PoolRate,
} from '../api'
import { fmtMoney, fmtPercent, fmtPercentShort } from '../format'
import InlineForm from '../components/InlineForm.vue'
import { numOrNull } from '../parse'
import { usePage } from '../page'

const settings = ref<AppSettings | null>(null)
const accounts = ref<CashAccount[]>([])
const assets = ref<ManualAsset[]>([])

const { message, error, reload, run } = usePage(load)

async function load() {
  const [settingsData, assetsData, accountsData] = await Promise.all([
    api.getSettings(),
    api.listManualAssets(),
    api.listCashAccounts(),
  ])
  settings.value = settingsData
  assets.value = assetsData
  accounts.value = accountsData
}

type Section = 'general' | 'mm' | 'goals' | 'calendar'
const editingSection = ref<Section | null>(null)

// ----- 一般 -----

const salaryDraft = ref<string | number>('')
const billDraft = ref<string | number>('')
const targetDraft = ref<string | number>('')

function startGeneralEdit() {
  salaryDraft.value = settings.value?.salary ?? ''
  billDraft.value = settings.value?.bill_amount ?? ''
  const target = settings.value?.semi_liquid_target
  targetDraft.value = target === null || target === undefined ? '' : target * 100
  editingSection.value = 'general'
}

function saveGeneral() {
  run(async () => {
    const salary = numOrNull(salaryDraft.value)
    const bill = numOrNull(billDraft.value)
    if (bill !== null && bill < 0) {
      throw new Error('繳費金額不能是負數')
    }
    const targetPct = numOrNull(targetDraft.value)
    if (targetPct !== null && (targetPct < 0 || targetPct >= 100)) {
      throw new Error('半流動資金目標必須是 0–100%')
    }
    await api.updateSettings({
      salary,
      // Blank resets to the 2158 default / the 25% sheet literal.
      bill_amount: bill,
      semi_liquid_target: targetPct === null ? null : targetPct / 100,
    })
    editingSection.value = null
  }, { message: '已儲存設定' })
}

// ----- 開心Pool -----

const editingRate = ref<PoolRate | 'new' | null>(null)
const rateDraft = ref<{ year: string | number; rate: string | number }>({
  year: '',
  rate: '',
})

function startRateEdit(row: PoolRate | 'new') {
  editingRate.value = row
  rateDraft.value =
    row === 'new'
      ? { year: settings.value?.pool_rate_year ?? new Date().getFullYear(), rate: '' }
      : { year: row.year, rate: row.rate * 100 }
}

function saveRate() {
  const wasNew = editingRate.value === 'new'
  run(async () => {
    const year = numOrNull(rateDraft.value.year)
    if (year === null || !Number.isInteger(year)) {
      throw new Error('年份必須是整數')
    }
    const pct = numOrNull(rateDraft.value.rate)
    if (pct === null || pct < 0 || pct >= 100) {
      throw new Error('開心Pool 利率必須是 0–100%')
    }
    await api.updateSettings({ pool_rate: pct / 100, pool_rate_year: year })
    editingRate.value = null
  }, { message: wasNew ? '已新增利率' : '已儲存利率' })
}

function removeRate(row: PoolRate) {
  // A cleared rate falls back to the latest earlier stored year.
  run(() => api.updateSettings({ pool_rate: null, pool_rate_year: row.year }), {
    confirm: `刪除 ${row.year} 年利率？`,
    message: `已刪除 ${row.year} 年利率`,
  })
}

// ----- Money Master -----

const mmStartDateDraft = ref('')
const mmSavedDraft = ref<string | number>('')
const mmTargetMonthsDraft = ref<string | number>('')
const mmTargetAmountDraft = ref<string | number>('')
const mmMonthNowDraft = ref<string | number>('')
const mmComingSaveDraft = ref<string | number>('')

function startMmEdit() {
  mmStartDateDraft.value = settings.value?.money_master_start_date ?? ''
  mmSavedDraft.value = settings.value?.money_master_saved ?? ''
  mmTargetMonthsDraft.value = settings.value?.money_master_target_months ?? ''
  mmTargetAmountDraft.value = settings.value?.money_master_target_amount ?? ''
  mmMonthNowDraft.value = settings.value?.money_master_month_now ?? ''
  mmComingSaveDraft.value = settings.value?.money_master_coming_save ?? ''
  editingSection.value = 'mm'
}

function saveMm() {
  run(async () => {
    const mmTargetMonths = numOrNull(mmTargetMonthsDraft.value)
    const mmMonthNow = numOrNull(mmMonthNowDraft.value)
    for (const value of [mmTargetMonths, mmMonthNow]) {
      if (value !== null && (value <= 0 || !Number.isInteger(value))) {
        throw new Error('Money Master 月數必須是正整數')
      }
    }
    const mmTargetAmount = numOrNull(mmTargetAmountDraft.value)
    if (mmTargetAmount !== null && mmTargetAmount <= 0) {
      throw new Error('Money Master 目標金額必須是正數')
    }
    await api.updateSettings({
      // Blank clears; the two overrides derive while unset.
      money_master_start_date: mmStartDateDraft.value || null,
      money_master_saved: numOrNull(mmSavedDraft.value),
      money_master_target_months: mmTargetMonths,
      money_master_target_amount: mmTargetAmount,
      money_master_month_now: mmMonthNow,
      money_master_coming_save: numOrNull(mmComingSaveDraft.value),
    })
    editingSection.value = null
  }, { message: '已儲存設定' })
}

// ----- 目標 -----

const goal1Draft = ref<string | number>('')
const goal2Draft = ref<string | number>('')
const goal3InflationDraft = ref<string | number>('')
const goal3YearsDraft = ref<string | number>('')
const finalReplacementDraft = ref<string | number>('')

function startGoalsEdit() {
  goal1Draft.value = settings.value?.goal1_amount ?? ''
  goal2Draft.value = settings.value?.goal2_amount ?? ''
  goal3InflationDraft.value = settings.value?.goal3_inflation ?? ''
  goal3YearsDraft.value = settings.value?.goal3_years ?? ''
  finalReplacementDraft.value = settings.value?.final_replacement ?? ''
  editingSection.value = 'goals'
}

function saveGoals() {
  run(async () => {
    const goal1 = numOrNull(goal1Draft.value)
    const goal2 = numOrNull(goal2Draft.value)
    for (const amount of [goal1, goal2]) {
      if (amount !== null && amount < 0) {
        throw new Error('目標金額不能是負數')
      }
    }
    const goal3Inflation = numOrNull(goal3InflationDraft.value)
    const goal3Years = numOrNull(goal3YearsDraft.value)
    const finalReplacement = numOrNull(finalReplacementDraft.value)
    for (const value of [goal3Inflation, goal3Years, finalReplacement]) {
      if (value !== null && value <= 0) {
        throw new Error('目標常數必須是正數')
      }
    }
    await api.updateSettings({
      // Blank clears the amounts and resets the constants to the sheet
      // literals (1.07 / 30 / 0.7).
      goal1_amount: goal1,
      goal2_amount: goal2,
      goal3_inflation: goal3Inflation,
      goal3_years: goal3Years,
      final_replacement: finalReplacement,
    })
    editingSection.value = null
  }, { message: '已儲存設定' })
}

// ----- 日曆 -----

const privateCalendarDraft = ref('')

function startCalendarEdit() {
  privateCalendarDraft.value = settings.value?.calendar_private_id ?? ''
  editingSection.value = 'calendar'
}

function saveCalendar() {
  run(async () => {
    await api.updateSettings({
      calendar_private_id: privateCalendarDraft.value.trim() || null,
    })
    editingSection.value = null
  }, { message: '已儲存設定' })
}

// ----- 資產 -----

const editingAsset = ref<ManualAsset | 'new' | null>(null)
const assetDraft = ref<{
  label: string
  liquidity: ManualAssetLiquidity
  amount: string | number
  note: string
}>({
  label: '',
  liquidity: 'long',
  amount: '',
  note: '',
})

function startAssetEdit(asset: ManualAsset | 'new') {
  editingAsset.value = asset
  assetDraft.value =
    asset === 'new'
      ? { label: '', liquidity: 'long', amount: '', note: '' }
      : {
          label: asset.label,
          liquidity: asset.liquidity,
          amount: asset.amount,
          note: asset.note ?? '',
        }
}

function saveAsset() {
  const wasNew = editingAsset.value === 'new'
  const target = editingAsset.value
  run(async () => {
    const amount = numOrNull(assetDraft.value.amount)
    if (assetDraft.value.label.trim() === '' || amount === null) {
      throw new Error('名稱及金額必填')
    }
    const body: NewManualAsset = {
      label: assetDraft.value.label.trim(),
      liquidity: assetDraft.value.liquidity,
      amount,
      note: assetDraft.value.note.trim() || null,
    }
    if (wasNew) {
      await api.createManualAsset(body)
    } else if (target && target !== 'new') {
      await api.updateManualAsset(target.id, body)
    }
    editingAsset.value = null
  }, {
    message: wasNew ? '已新增結餘' : '已儲存結餘',
  })
}

function removeAsset(asset: ManualAsset) {
  run(() => api.deleteManualAsset(asset.id), {
    confirm: `刪除結餘 ${asset.label}？`,
    message: `已刪除 ${asset.label}`,
  })
}

// ----- 活期帳戶 -----

const editingAccount = ref<CashAccount | 'new' | null>(null)
const accountDraft = ref<{
  label: string
  currency: Currency
  role: CashRole
  amount: string | number
  is_default_receipt: boolean
}>({
  label: '',
  currency: 'HKD',
  role: 'bank',
  amount: '',
  is_default_receipt: false,
})

// Bank accounts are HKD-only; snap the currency back when the role flips.
watch(
  () => accountDraft.value.role,
  (role) => {
    if (role === 'bank') accountDraft.value.currency = 'HKD'
  },
)

function startAccountEdit(account: CashAccount | 'new') {
  editingAccount.value = account
  accountDraft.value =
    account === 'new'
      ? { label: '', currency: 'HKD', role: 'bank', amount: '', is_default_receipt: false }
      : {
          label: account.label,
          currency: account.currency,
          role: account.role,
          amount: account.amount,
          is_default_receipt: account.is_default_receipt,
        }
}

function saveAccount() {
  const wasNew = editingAccount.value === 'new'
  const target = editingAccount.value
  run(async () => {
    const amount = numOrNull(accountDraft.value.amount)
    if (accountDraft.value.label.trim() === '' || amount === null) {
      throw new Error('名稱及金額必填')
    }
    const body: NewCashAccount = {
      label: accountDraft.value.label.trim(),
      currency: accountDraft.value.currency,
      role: accountDraft.value.role,
      amount,
      is_default_receipt: accountDraft.value.is_default_receipt,
    }
    if (wasNew) {
      await api.createCashAccount(body)
    } else if (target && target !== 'new') {
      await api.updateCashAccount(target.id, body)
    }
    editingAccount.value = null
  }, {
    message: wasNew ? '已新增帳戶' : '已儲存帳戶',
  })
}

function removeAccount(account: CashAccount) {
  // 409 (broker rows, or other server-side refusals) surfaces the server message.
  run(() => api.deleteCashAccount(account.id), {
    confirm: `刪除帳戶 ${account.label}？`,
    message: `已刪除 ${account.label}`,
  })
}

onMounted(reload)
</script>

<template>
  <section>
    <div class="card">
      <h3>活期帳戶</h3>
      <table>
        <thead>
          <tr>
            <th>名稱</th>
            <th>幣別</th>
            <th>類型</th>
            <th class="num">金額</th>
            <th>預設收款</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="account in accounts" :key="account.id">
            <td>{{ account.label }}</td>
            <td>{{ account.currency }}</td>
            <td>{{ account.role === 'bank' ? '銀行' : '券商' }}</td>
            <td class="num">{{ fmtMoney(account.amount) }}</td>
            <td>{{ account.is_default_receipt ? '✓' : '' }}</td>
            <td class="row-actions">
              <button type="button" class="link" @click="startAccountEdit(account)">編輯</button>
              <button
                v-if="account.role !== 'broker'"
                type="button"
                class="link danger"
                @click="removeAccount(account)"
              >
                刪除
              </button>
            </td>
          </tr>
          <tr v-if="accounts.length === 0">
            <td colspan="6" class="muted">沒有活期帳戶</td>
          </tr>
        </tbody>
      </table>
      <button type="button" class="link" @click="startAccountEdit('new')">新增帳戶</button>

      <InlineForm
        v-if="editingAccount"
        :title="editingAccount === 'new' ? '新增帳戶' : `編輯 ${editingAccount.label}`"
        @submit="saveAccount"
        @cancel="editingAccount = null"
      >
        <label>
          名稱
          <input v-model="accountDraft.label" required />
        </label>
        <label>
          幣別
          <select v-model="accountDraft.currency">
            <option value="HKD">HKD</option>
            <option value="USD" :disabled="accountDraft.role === 'bank'">USD</option>
          </select>
        </label>
        <label>
          類型
          <select v-model="accountDraft.role">
            <option value="bank">銀行</option>
            <option value="broker">券商</option>
          </select>
        </label>
        <label>
          金額
          <input
            v-model="accountDraft.amount"
            type="number"
            step="any"
            inputmode="decimal"
            required
          />
        </label>
        <label class="checkbox">
          <input v-model="accountDraft.is_default_receipt" type="checkbox" />
          預設收款
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>資產</h3>
      <table>
        <thead>
          <tr>
            <th>名稱</th>
            <th>流動性</th>
            <th class="num">金額</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="asset in assets" :key="asset.id">
            <td>
              {{ asset.label }}
              <small v-if="asset.note" class="muted note">{{ asset.note }}</small>
            </td>
            <td>{{ asset.liquidity === 'short' ? '短期' : '長期' }}</td>
            <td class="num">{{ fmtMoney(asset.amount) }}</td>
            <td class="row-actions">
              <button type="button" class="link" @click="startAssetEdit(asset)">編輯</button>
              <button type="button" class="link danger" @click="removeAsset(asset)">刪除</button>
            </td>
          </tr>
          <tr v-if="assets.length === 0">
            <td colspan="4" class="muted">沒有手動結餘</td>
          </tr>
        </tbody>
      </table>
      <button type="button" class="link" @click="startAssetEdit('new')">新增結餘</button>

      <InlineForm
        v-if="editingAsset"
        :title="editingAsset === 'new' ? '新增結餘' : `編輯 ${editingAsset.label}`"
        @submit="saveAsset"
        @cancel="editingAsset = null"
      >
        <label>
          名稱
          <input v-model="assetDraft.label" required />
        </label>
        <label>
          流動性
          <select v-model="assetDraft.liquidity">
            <option value="short">短期</option>
            <option value="long">長期</option>
          </select>
        </label>
        <label>
          金額
          <input v-model="assetDraft.amount" type="number" step="any" inputmode="decimal" required />
        </label>
        <label>
          備註
          <input v-model="assetDraft.note" />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>一般</h3>
      <p class="muted settings-line">
        薪金 {{ fmtMoney(settings?.salary) || '—' }} · 繳費
        {{ fmtMoney(settings?.bill_amount) || '—' }} · 流動目標
        {{ fmtPercentShort(settings?.semi_liquid_target) || '—' }}
        <button type="button" class="link" @click="startGeneralEdit">編輯</button>
      </p>
      <InlineForm
        v-if="editingSection === 'general'"
        title="一般設定"
        @submit="saveGeneral"
        @cancel="editingSection = null"
      >
        <label>
          薪金
          <input v-model="salaryDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          預測繳費（季繳差餉；留空回復 2158）
          <input v-model="billDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          半流動資金目標（流動資產 %；留空回復 25%）
          <input v-model="targetDraft" type="number" step="any" inputmode="decimal" placeholder="25" />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>開心Pool</h3>
      <table>
        <thead>
          <tr>
            <th>年份</th>
            <th class="num">利率</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in settings?.pool_rates ?? []" :key="row.year">
            <td>{{ row.year }}</td>
            <td class="num">{{ fmtPercent(row.rate) }}</td>
            <td class="row-actions">
              <button type="button" class="link" @click="startRateEdit(row)">編輯</button>
              <button type="button" class="link danger" @click="removeRate(row)">刪除</button>
            </td>
          </tr>
          <tr v-if="(settings?.pool_rates ?? []).length === 0">
            <td colspan="3" class="muted">沒有利率記錄</td>
          </tr>
        </tbody>
      </table>
      <button type="button" class="link" @click="startRateEdit('new')">新增利率</button>
      <p class="muted settings-line">
        今年 {{ settings?.pool_rate_year ?? '—' }} 利率
        {{ fmtPercent(settings?.pool_rate) || '—' }} — 未設年份沿用最近較早年份
      </p>

      <InlineForm
        v-if="editingRate"
        :title="editingRate === 'new' ? '新增利率' : `編輯 ${editingRate.year} 利率`"
        @submit="saveRate"
        @cancel="editingRate = null"
      >
        <label>
          年份
          <input
            v-model="rateDraft.year"
            type="number"
            step="1"
            inputmode="numeric"
            :disabled="editingRate !== 'new'"
            required
          />
        </label>
        <label>
          利率（%）
          <input
            v-model="rateDraft.rate"
            type="number"
            step="any"
            inputmode="decimal"
            placeholder="33.7"
            required
          />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>Money Master</h3>
      <p class="muted settings-line">
        開始 {{ settings?.money_master_start_date || '—' }} · 已儲
        {{ fmtMoney(settings?.money_master_saved) || '—' }} · 目標
        {{ settings?.money_master_target_months ?? '—' }}月/{{
          fmtMoney(settings?.money_master_target_amount) || '—'
        }}
        <template
          v-if="
            settings?.money_master_month_now != null ||
            settings?.money_master_coming_save != null
          "
        >
          · 覆寫 {{ settings?.money_master_month_now ?? '—' }}月/{{
            fmtMoney(settings?.money_master_coming_save) || '—'
          }}
        </template>
        <button type="button" class="link" @click="startMmEdit">編輯</button>
      </p>
      <InlineForm
        v-if="editingSection === 'mm'"
        title="Money Master"
        @submit="saveMm"
        @cancel="editingSection = null"
      >
        <label>
          挑戰開始日
          <input v-model="mmStartDateDraft" type="date" />
        </label>
        <label>
          已儲（銀行 App 數字）
          <input v-model="mmSavedDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          目標月數
          <input v-model="mmTargetMonthsDraft" type="number" step="1" inputmode="numeric" />
        </label>
        <label>
          目標金額
          <input v-model="mmTargetAmountDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          月份覆寫（留空按開始日推算）
          <input v-model="mmMonthNowDraft" type="number" step="1" inputmode="numeric" />
        </label>
        <label>
          每月需儲覆寫（留空自動推算）
          <input v-model="mmComingSaveDraft" type="number" step="any" inputmode="decimal" />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>目標</h3>
      <p class="muted settings-line">
        1st {{ fmtMoney(settings?.goal1_amount) || '—' }} · 2nd
        {{ fmtMoney(settings?.goal2_amount) || '—' }} · 3rd ×{{
          settings?.goal3_inflation ?? '—'
        }}/{{ settings?.goal3_years ?? '—' }}年 · Final
        {{ fmtPercentShort(settings?.final_replacement) || '—' }}
        <button type="button" class="link" @click="startGoalsEdit">編輯</button>
      </p>
      <InlineForm
        v-if="editingSection === 'goals'"
        title="目標"
        @submit="saveGoals"
        @cancel="editingSection = null"
      >
        <label>
          1st 目標金額（留空清除）
          <input v-model="goal1Draft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          2nd 目標金額（留空清除）
          <input v-model="goal2Draft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          3rd 通脹倍數（留空回復 1.07）
          <input v-model="goal3InflationDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          3rd 年數（留空回復 30）
          <input v-model="goal3YearsDraft" type="number" step="1" inputmode="numeric" />
        </label>
        <label>
          Final 薪金替代率（留空回復 0.7）
          <input
            v-model="finalReplacementDraft"
            type="number"
            step="any"
            inputmode="decimal"
          />
        </label>
      </InlineForm>
    </div>

    <div class="card">
      <h3>日曆</h3>
      <p class="muted settings-line">
        Private {{ settings?.calendar_private_id || '—' }}
        <button type="button" class="link" @click="startCalendarEdit">編輯</button>
      </p>
      <InlineForm
        v-if="editingSection === 'calendar'"
        title="日曆"
        @submit="saveCalendar"
        @cancel="editingSection = null"
      >
        <label>
          Private 日曆 ID
          <input
            v-model="privateCalendarDraft"
            type="text"
            placeholder="…@group.calendar.google.com"
          />
        </label>
      </InlineForm>
    </div>

    <p v-if="message" class="ok">{{ message }}</p>
    <p v-if="error" class="error">{{ error?.message }}</p>
  </section>
</template>

<style scoped>
.note {
  display: block;
  font-size: 0.75rem;
  font-family: monospace;
  white-space: normal;
}
.settings-line {
  font-size: 0.85rem;
}
</style>
