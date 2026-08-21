<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useData } from 'vitepress'
import AppearancePanel from './AppearancePanel.vue'

const { lang } = useData()
const label = computed(() =>
  lang.value.startsWith('en') ? 'Appearance settings' : 'Настройки внешнего вида',
)

const open = ref(false)
const root = ref<HTMLElement | null>(null)

function onDocClick(event: MouseEvent): void {
  if (root.value && !root.value.contains(event.target as Node)) {
    open.value = false
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    open.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onKeydown)
})

watch(open, (isOpen) => {
  if (isOpen) {
    nextTick(() => {
      root.value
        ?.querySelector<HTMLInputElement>('.den-radio-input')
        ?.focus()
    })
  }
})
</script>

<template>
  <div ref="root" class="appearance-trigger">
    <button
      type="button"
      class="appearance-trigger-btn"
      :aria-expanded="open"
      :aria-label="label"
      @click="open = !open"
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M4 21v-7" />
        <path d="M4 10V3" />
        <path d="M12 21v-9" />
        <path d="M12 8V3" />
        <path d="M20 21v-5" />
        <path d="M20 12V3" />
        <path d="M1 14h6" />
        <path d="M9 8h6" />
        <path d="M17 16h6" />
      </svg>
    </button>

    <Transition name="den-pop">
      <div
        v-show="open"
        class="appearance-pop"
        role="dialog"
        :aria-label="label"
      >
        <AppearancePanel />
      </div>
    </Transition>
  </div>
</template>