<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  api,
  type AppSettings,
  type CashAccount,
  type CashRole,
  type Currency,
  type ManualAsset,
  type ManualAssetLiquidity,
  type MonthSummary,
  type NewCashAccount,
  type NewManualAsset,
} from '../api'
import { fmtMoney, fmtPercent, fmtPercentShort } from '../format'
import { numOrNull } from '../parse'
import type { PageRun } from '../page'
const props = defineProps<{
  settings: AppSettings | null
  summary: MonthSummary | null
  accounts: CashAccount[]
  assets: ManualAsset[]
  /** The displayed year — labels the pool rate and becomes `pool_rate_year`. */
  year: number
  /** The page's mutation runner — saves land on the page's message/error. */
  run: PageRun
}>()

const settingsOpen = ref(false)
const salaryDraft = ref<string | number>('')
const rateDraft = ref<string | number>('')
const billDraft = ref<string | number>('')
const targetDraft = ref<string | number>('')
const mmStartDateDraft = ref('')
const mmSavedDraft = ref<string | number>('')
const mmTargetMonthsDraft = ref<string | number>('')
const mmTargetAmountDraft = ref<string | number>('')
const mmMonthNowDraft = ref<string | number>('')
const mmComingSaveDraft = ref<string | number>('')
const goal1Draft = ref<string | number>('')
const goal2Draft = ref<string | number>('')
const goal3InflationDraft = ref<string | number>('')
const goal3YearsDraft = ref<string | number>('')
const finalReplacementDraft = ref<string | number>('')
const privateCalendarDraft = ref('')
const editingSettings = ref(false)

function startSettingsEdit() {
  salaryDraft.value = props.settings?.salary ?? ''
  // Stored as a fraction; edited as a percent, same as deposit rates.
  const rate = props.summary?.pool_rate
  rateDraft.value = rate === null || rate === undefined ? '' : rate * 100
  billDraft.value = props.settings?.bill_amount ?? ''
  const target = props.settings?.semi_liquid_target
  targetDraft.value = target === null || target === undefined ? '' : target * 100
  mmStartDateDraft.value = props.settings?.money_master_start_date ?? ''
  mmSavedDraft.value = props.settings?.money_master_saved ?? ''
  mmTargetMonthsDraft.value = props.settings?.money_master_target_months ?? ''
  mmTargetAmountDraft.value = props.settings?.money_master_target_amount ?? ''
  mmMonthNowDraft.value = props.settings?.money_master_month_now ?? ''
  mmComingSaveDraft.value = props.settings?.money_master_coming_save ?? ''
  goal1Draft.value = props.settings?.goal1_amount ?? ''
  goal2Draft.value = props.settings?.goal2_amount ?? ''
  goal3InflationDraft.value = props.settings?.goal3_inflation ?? ''
  goal3YearsDraft.value = props.settings?.goal3_years ?? ''
  finalReplacementDraft.value = props.settings?.final_replacement ?? ''
  privateCalendarDraft.value = props.settings?.calendar_private_id ?? ''
  editingSettings.value = true
}

function saveSettings() {
  props.run(async () => {
    const salary = numOrNull(salaryDraft.value)
    const ratePct = numOrNull(rateDraft.value)
    if (ratePct !== null && (ratePct < 0 || ratePct >= 100)) {
      throw new Error('開心Pool 利率必須是 0–100%')
    }
    const bill = numOrNull(billDraft.value)
    if (bill !== null && bill < 0) {
      throw new Error('繳費金額不能是負數')
    }
    const targetPct = numOrNull(targetDraft.value)
    if (targetPct !== null && (targetPct < 0 || targetPct >= 100)) {
      throw new Error('半流動資金目標必須是 0–100%')
    }
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
      salary,
      pool_rate: ratePct === null ? null : ratePct / 100,
      pool_rate_year: props.year,
      // Empty keeps the stored value? No — the forecast reads the effective
      // amount; blank resets to the 2158 default.
      bill_amount: bill,
      // Same: blank resets to the 25% sheet literal.
      semi_liquid_target: targetPct === null ? null : targetPct / 100,
      // Money Master: blank clears; the two overrides derive while unset.
      money_master_start_date: mmStartDateDraft.value || null,
      money_master_saved: numOrNull(mmSavedDraft.value),
      money_master_target_months: mmTargetMonths,
      money_master_target_amount: mmTargetAmount,
      money_master_month_now: mmMonthNow,
      money_master_coming_save: numOrNull(mmComingSaveDraft.value),
      // 目標: blank clears the amounts and resets the constants to the
      // sheet literals (1.07 / 30 / 0.7).
      goal1_amount: goal1,
      goal2_amount: goal2,
      goal3_inflation: goal3Inflation,
      goal3_years: goal3Years,
      final_replacement: finalReplacement,
      calendar_private_id: privateCalendarDraft.value.trim() || null,
    })
    editingSettings.value = false
  }, { message: '已儲存設定' })
}

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
  props.run(async () => {
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
  props.run(() => api.deleteManualAsset(asset.id), {
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
  props.run(async () => {
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
  props.run(() => api.deleteCashAccount(account.id), {
    confirm: `刪除帳戶 ${account.label}？`,
    message: `已刪除 ${account.label}`,
  })
}
</script>

<template>
  <div class="card">
    <h3>
      <button type="button" class="link" @click="settingsOpen = !settingsOpen">
        設定 {{ settingsOpen ? '▾' : '▸' }}
      </button>
    </h3>
    <template v-if="settingsOpen">
      <h4>活期帳戶</h4>
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

      <form v-if="editingAccount" class="inline-form" @submit.prevent="saveAccount">
        <h4>{{ editingAccount === 'new' ? '新增帳戶' : `編輯 ${editingAccount.label}` }}</h4>
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
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingAccount = null">取消</button>
        </div>
      </form>

      <h4>資產</h4>
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

      <form v-if="editingAsset" class="inline-form" @submit.prevent="saveAsset">
        <h4>{{ editingAsset === 'new' ? '新增結餘' : `編輯 ${editingAsset.label}` }}</h4>
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
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingAsset = null">取消</button>
        </div>
      </form>

      <p class="muted settings-line">
        薪金 {{ fmtMoney(settings?.salary) || '—' }} · 開心Pool {{ year }} 利率
        {{ fmtPercent(year === summary?.pool_rate_year ? summary?.pool_rate : null) || '—' }} ·
        繳費 {{ fmtMoney(settings?.bill_amount) || '—' }} · 流動目標
        {{ fmtPercentShort(settings?.semi_liquid_target) || '—' }}
        <button type="button" class="link" @click="startSettingsEdit">編輯</button>
      </p>
      <p class="muted settings-line">
        Money Master 開始 {{ settings?.money_master_start_date || '—' }} · 已儲
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
      </p>
      <p class="muted settings-line">
        目標 1st {{ fmtMoney(settings?.goal1_amount) || '—' }} · 2nd
        {{ fmtMoney(settings?.goal2_amount) || '—' }} · 3rd ×{{
          settings?.goal3_inflation ?? '—'
        }}/{{ settings?.goal3_years ?? '—' }}年 · Final
        {{ fmtPercentShort(settings?.final_replacement) || '—' }}
      </p>
      <p class="muted settings-line">
        日曆 Private {{ settings?.calendar_private_id || '—' }}
      </p>
      <form v-if="editingSettings" class="inline-form" @submit.prevent="saveSettings">
        <h4>設定</h4>
        <label>
          薪金
          <input v-model="salaryDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          開心Pool 利率（{{ year }}，%）
          <input v-model="rateDraft" type="number" step="any" inputmode="decimal" placeholder="33.7" />
        </label>
        <label>
          預測繳費（季繳差餉；留空回復 2158）
          <input v-model="billDraft" type="number" step="any" inputmode="decimal" />
        </label>
        <label>
          半流動資金目標（流動資產 %；留空回復 25%）
          <input v-model="targetDraft" type="number" step="any" inputmode="decimal" placeholder="25" />
        </label>
        <h4>Money Master</h4>
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
        <h4>目標</h4>
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
        <h4>日曆</h4>
        <label>
          Private 日曆 ID
          <input
            v-model="privateCalendarDraft"
            type="text"
            placeholder="…@group.calendar.google.com"
          />
        </label>
        <div class="form-actions">
          <button type="submit">儲存</button>
          <button type="button" class="link" @click="editingSettings = false">取消</button>
        </div>
      </form>
    </template>
  </div>
</template>

<style scoped>
.note {
  display: block;
  font-size: 0.75rem;
  font-family: monospace;
  white-space: normal;
}
.inline-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
}
.inline-form h4 {
  grid-column: 1 / -1;
  margin: 0;
}
.inline-form label {
  display: flex;
  flex-direction: column;
  font-size: 0.8rem;
  gap: 0.2rem;
}
.form-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
.settings-line {
  font-size: 0.85rem;
}
</style>
