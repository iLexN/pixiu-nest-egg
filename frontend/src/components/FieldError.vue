<script setup lang="ts">
import { computed } from 'vue'
import type { ApiError } from '../api'

const props = defineProps<{ error: ApiError | null; field: string }>()

const message = computed(() => props.error?.fieldMessage(props.field))
</script>

<template>
  <small v-if="message" class="error">{{ message }}</small>
</template>
