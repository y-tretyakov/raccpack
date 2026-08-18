<script setup lang="ts">
import { withBase } from 'vitepress'
</script>

<template>
  <a
    class="sidebar-brand"
    :href="withBase('/')"
    aria-label="raccpack — главная страница"
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
