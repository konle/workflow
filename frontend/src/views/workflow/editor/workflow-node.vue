<template>
  <div class="wf-node" :class="{ dangling: data.dangling }" :style="nodeStyle">
    <Handle type="target" :position="Position.Top" class="handle-target" />
    <div class="wf-node-label">{{ data.label }}</div>
    <Handle type="source" :position="Position.Bottom" class="handle-source" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'

const props = defineProps<{ data: any }>()

const nodeStyle = computed(() => ({
  background: props.data.color || '#86909C',
  color: '#fff',
  borderRadius: '6px',
  padding: '6px 14px',
  fontSize: '12px',
  border: props.data.dangling ? '2px solid #F53F3F' : '2px solid transparent',
  minWidth: '120px',
  textAlign: 'center' as const,
}))
</script>

<style scoped>
.wf-node {
  position: relative;
}
.wf-node.dangling {
  animation: dangling-pulse 1s ease-in-out infinite;
}
@keyframes dangling-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(245, 63, 63, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(245, 63, 63, 0); }
}
.wf-node-label {
  white-space: nowrap;
}
.handle-target, .handle-source {
  width: 8px;
  height: 8px;
  background: #fff;
  border: 2px solid #86909C;
  border-radius: 50%;
}
</style>
