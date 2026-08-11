<script setup lang="ts">
import { withBase } from 'vitepress'
</script>

<template>
  <a
    class="sidebar-brand"
    :href="withBase('/')"
    aria-label="RaccPack — главная страница"
  >
    <img class="sidebar-brand-img" :src="withBase('/RaccPack.webp')" alt="RaccPack" />
  </a>
</template>