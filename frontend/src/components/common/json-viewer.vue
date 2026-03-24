<template>
  <pre class="json-viewer">{{ formatted }}</pre>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ data: unknown }>()

const formatted = computed(() => {
  if (props.data == null) return 'null'
  try {
    return JSON.stringify(props.data, null, 2)
  } catch {
    return String(props.data)
  }
})
</script>

<style scoped>
.json-viewer {
  background: var(--color-fill-2);
  border: 1px solid var(--color-border);
  border-radius: 4px;
  padding: 12px;
  font-size: 13px;
  line-height: 1.6;
  overflow: auto;
  max-height: 400px;
  margin: 0;
}
</style>
