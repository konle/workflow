<template>
  <a-tag :color="statusInfo.color" size="small">{{ statusInfo.label }}</a-tag>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  status: string
  map: Record<string, { label: string; color: string }>
}>()

const statusInfo = computed(() => {
  return props.map[props.status] || { label: props.status, color: 'gray' }
})
</script>
