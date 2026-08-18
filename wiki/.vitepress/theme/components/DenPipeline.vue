<script setup lang="ts">
const steps = ['sniff', 'dig', 'stash', 'rinse', 'pack', 'raid'] as const
</script>

<template>
  <div class="den-pipeline" role="list" aria-label="Пайплайн команд raccpack">
    <template v-for="(step, i) in steps" :key="step">
      <span class="step" role="listitem">{{ step }}</span>
      <span v-if="i < steps.length - 1" class="arrow" aria-hidden="true">→</span>
    </template>
  </div>
</template>
