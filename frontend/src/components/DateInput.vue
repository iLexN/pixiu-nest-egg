<script setup lang="ts">
import { ref } from 'vue'

defineOptions({ inheritAttrs: false })

const model = defineModel<string>({ default: '' })

const emit = defineEmits<{ change: [] }>()

const picker = ref<HTMLInputElement | null>(null)

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/

function openPicker() {
  const el = picker.value
  if (!el) return
  el.value = ISO_DATE.test(model.value) ? model.value : ''
  el.showPicker()
}

function onPick(event: Event) {
  const value = (event.target as HTMLInputElement).value
  if (value) {
    model.value = value
    emit('change')
  }
}
</script>

<template>
  <span class="date-input">
    <input
      v-model="model"
      v-bind="$attrs"
      type="text"
      inputmode="numeric"
      placeholder="YYYY-MM-DD"
      pattern="\d{4}-\d{2}-\d{2}"
      maxlength="10"
      autocomplete="off"
      @change="emit('change')"
    />
    <button
      type="button"
      class="picker-button"
      aria-label="選擇日期"
      tabindex="-1"
      @click="openPicker"
    >
      📅
    </button>
    <input
      ref="picker"
      type="date"
      class="hidden-picker"
      tabindex="-1"
      aria-hidden="true"
      @change="onPick"
    />
  </span>
</template>

<style scoped>
.date-input {
  position: relative;
  display: inline-flex;
  width: 100%;
}
.date-input input[type='text'] {
  flex: 1;
  min-width: 0;
  padding-right: 1.5rem;
}
.picker-button {
  position: absolute;
  right: 0.25rem;
  top: 50%;
  transform: translateY(-50%);
  border: none;
  background: none;
  padding: 0;
  font-size: 0.8rem;
  line-height: 1;
  cursor: pointer;
}
.hidden-picker {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 1.5rem;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}
</style>
