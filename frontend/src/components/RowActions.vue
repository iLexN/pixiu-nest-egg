<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'

const emit = defineEmits<{
  edit: []
  remove: []
}>()

const open = ref(false)

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') open.value = false
}

function toggle() {
  open.value = !open.value
  if (open.value) {
    document.addEventListener('keydown', onKeydown)
  } else {
    document.removeEventListener('keydown', onKeydown)
  }
}

function close() {
  open.value = false
  document.removeEventListener('keydown', onKeydown)
}

function pick(action: 'edit' | 'remove') {
  close()
  if (action === 'edit') emit('edit')
  else emit('remove')
}

onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="row-menu">
    <button type="button" class="link" aria-label="操作" @click="toggle">⋯</button>
    <template v-if="open">
      <div class="backdrop" @click="close"></div>
      <div class="menu">
        <button type="button" class="link" @click="pick('edit')">編輯</button>
        <button type="button" class="link danger" @click="pick('remove')">刪除</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.row-menu {
  position: relative;
  display: inline-block;
}
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 10;
}
.menu {
  position: absolute;
  right: 0;
  top: 100%;
  z-index: 11;
  display: flex;
  flex-direction: column;
  min-width: 4rem;
  padding: 0.15rem 0;
  background: var(--card);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  box-shadow: 0 2px 6px rgb(41 37 36 / 15%);
}
.menu .link {
  width: 100%;
  padding: 0.3rem 0.75rem;
  text-align: left;
}
</style>
