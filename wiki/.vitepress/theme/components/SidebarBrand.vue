<script setup lang="ts">
import { computed } from 'vue'
import { useData, withBase } from 'vitepress'

const { lang } = useData()
const brandLabel = computed(() =>
  lang.value.startsWith('en') ? 'raccpack — home page' : 'raccpack — главная страница',
)
const brandHref = computed(() => withBase(lang.value.startsWith('en') ? '/' : '/ru/'))
</script>

<template>
  <a
    class="sidebar-brand"
    :href="brandHref"
    :aria-label="brandLabel"
  >
    <img
      class="sidebar-brand-img"
      :src="withBase('/RaccPack.webp')"
      alt="raccpack"
      width="240"
      height="240"
    />
  </a>
</template>
