<template>
  <div class="cond-node" :class="{ dangling: data.dangling, 'config-error': data.configError }" :style="nodeStyle">
    <Handle type="target" :position="Position.Top" class="handle-target" />
    <div class="cond-node-label">{{ data.label }}</div>
    <div class="cond-handles">
      <div class="cond-handle-wrap">
        <Handle id="then" type="source" :position="Position.Bottom" class="handle-then" :style="{ left: '25%' }" />
        <span class="handle-label then-label">True</span>
      </div>
      <div class="cond-handle-wrap">
        <Handle id="else" type="source" :position="Position.Bottom" class="handle-else" :style="{ left: '75%' }" />
        <span class="handle-label else-label">False</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'

const props = defineProps<{ data: any }>()

const nodeStyle = computed(() => ({
  background: props.data.color || '#F7BA1E',
  color: '#fff',
  borderRadius: '6px',
  padding: '6px 14px 18px',
  fontSize: '12px',
  border: props.data.dangling ? '2px solid #F53F3F' : props.data.configError ? '2px solid #FF7D00' : '2px solid transparent',
  minWidth: '140px',
  textAlign: 'center' as const,
  position: 'relative' as const,
}))
</script>

<style scoped>
.cond-node {
  position: relative;
}
.cond-node.dangling {
  animation: dangling-pulse 1s ease-in-out infinite;
}
.cond-node.config-error {
  animation: config-error-pulse 1s ease-in-out infinite;
}
@keyframes dangling-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(245, 63, 63, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(245, 63, 63, 0); }
}
@keyframes config-error-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(255, 125, 0, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(255, 125, 0, 0); }
}
.cond-node-label {
  white-space: nowrap;
}
.cond-handles {
  position: absolute;
  bottom: -4px;
  left: 0;
  right: 0;
  display: flex;
  justify-content: space-between;
  padding: 0 10px;
  pointer-events: none;
}
.cond-handle-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}
.handle-label {
  font-size: 9px;
  margin-top: 2px;
  font-weight: 600;
  pointer-events: none;
  user-select: none;
}
.then-label { color: #00B42A; }
.else-label { color: #F53F3F; }
.handle-target {
  width: 8px;
  height: 8px;
  background: #fff;
  border: 2px solid #86909C;
  border-radius: 50%;
}
.handle-then {
  width: 10px;
  height: 10px;
  background: #00B42A;
  border: 2px solid #fff;
  border-radius: 50%;
  pointer-events: all;
}
.handle-else {
  width: 10px;
  height: 10px;
  background: #F53F3F;
  border: 2px solid #fff;
  border-radius: 50%;
  pointer-events: all;
}
</style>
