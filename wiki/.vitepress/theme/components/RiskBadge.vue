<script setup lang="ts">
const props = defineProps<{
  level: 'critical' | 'high' | 'medium' | 'low' | 'info'
}>()

const labels: Record<typeof props.level, string> = {
  critical: 'Critical',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
  info: 'Info',
}
</script>

<template>
  <span class="risk-badge" :class="level">{{ labels[level] }}</span>
</template>
