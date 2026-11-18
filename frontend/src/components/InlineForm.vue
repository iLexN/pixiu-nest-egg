<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  title?: string
  submitLabel?: string
}>()

const emit = defineEmits<{ submit: []; cancel: [] }>()

const formEl = ref<HTMLFormElement | null>(null)
defineExpose({ formEl })
</script>

<template>
  <form ref="formEl" class="inline-form" @submit.prevent="emit('submit')">
    <h4 v-if="title">{{ title }}</h4>
    <slot />
    <div class="form-actions">
      <slot name="actions">
        <button type="submit">{{ submitLabel ?? '儲存' }}</button>
        <button type="button" class="link" @click="emit('cancel')">取消</button>
      </slot>
    </div>
  </form>
</template>

<style scoped>
.inline-form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
}
.inline-form :deep(h4) {
  grid-column: 1 / -1;
  margin: 0;
}
.inline-form :deep(label) {
  display: flex;
  flex-direction: column;
  font-size: 0.8rem;
  gap: 0.2rem;
}
.inline-form :deep(label.check),
.inline-form :deep(label.checkbox) {
  flex-direction: row;
  align-items: center;
  gap: 0.4rem;
}
.form-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
</style>
