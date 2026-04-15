<template>
  <div>
    <a-page-header title="任务实例详情" @back="$router.push('/tasks/instances')" />
    <a-card :loading="loading">
      <template #extra>
        <a-space>
          <a-button
            v-if="entity?.task_status === 'Pending' && canExecute"
            type="primary" size="small"
            :loading="operating"
            @click="handleExecute"
          >执行</a-button>
          <a-button
            v-if="entity?.task_status === 'Failed' && canExecute"
            size="small"
            :loading="operating"
            @click="handleRetry"
          >重试</a-button>
          <a-button
            v-if="entity && ['Pending','Failed'].includes(entity.task_status) && canExecute"
            size="small" status="danger"
            :loading="operating"
            @click="handleCancel"
          >取消</a-button>
        </a-space>
      </template>

      <a-descriptions :column="2" bordered>
        <a-descriptions-item label="实例ID">{{ entity?.task_instance_id }}</a-descriptions-item>
        <a-descriptions-item label="任务名称">{{ entity?.task_name }}</a-descriptions-item>
        <a-descriptions-item label="任务类型">
          <a-tag v-if="entity" :color="TASK_TYPE_MAP[entity.task_type]?.color">
            {{ TASK_TYPE_MAP[entity.task_type]?.label || entity.task_type }}
          </a-tag>
        </a-descriptions-item>
        <a-descriptions-item label="状态">
          <status-tag v-if="entity" :status="entity.task_status" :map="TASK_INSTANCE_STATUS_MAP" />
        </a-descriptions-item>
        <a-descriptions-item label="耗时">{{ formatDuration(entity?.execution_duration) }}</a-descriptions-item>
        <a-descriptions-item label="创建时间">{{ formatDate(entity?.created_at || '') }}</a-descriptions-item>
      </a-descriptions>
    </a-card>

    <a-row :gutter="16" style="margin-top: 16px">
      <a-col :span="12">
        <a-card title="Input">
          <json-viewer :data="entity?.input" />
        </a-card>
      </a-col>
      <a-col :span="12">
        <a-card title="Output">
          <json-viewer :data="entity?.output" />
        </a-card>
      </a-col>
    </a-row>

    <a-card v-if="entity?.error_message" title="错误信息" style="margin-top: 16px">
      <a-alert type="error" :title="entity.error_message" />
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { taskInstanceApi } from '../../../api/task-instance'
import { usePermission } from '../../../composables/use-permission'
import { TASK_INSTANCE_STATUS_MAP, TASK_TYPE_MAP } from '../../../utils/constants'
import { formatDate, formatDuration } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import JsonViewer from '../../../components/common/json-viewer.vue'
import { Notification } from '@arco-design/web-vue'
import type { TaskInstanceEntity } from '../../../types/task'

const route = useRoute()
const { canExecute } = usePermission()
const loading = ref(false)
const operating = ref(false)
const entity = ref<TaskInstanceEntity | null>(null)
let pollTimer: ReturnType<typeof setInterval> | null = null

const TERMINAL_STATUSES = ['Completed', 'Failed', 'Canceled', 'Skipped']

async function fetchDetail() {
  try {
    const res = await taskInstanceApi.get(route.params.id as string)
    entity.value = res.data
    if (entity.value && TERMINAL_STATUSES.includes(entity.value.task_status)) {
      stopPolling()
    }
  } catch {}
}

function startPolling() {
  if (pollTimer) return
  pollTimer = setInterval(fetchDetail, 3000)
}

function stopPolling() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
}

async function handleExecute() {
  if (!entity.value) return
  operating.value = true
  try {
    await taskInstanceApi.execute(entity.value.id)
    Notification.success({ content: '已提交执行' })
    startPolling()
    await fetchDetail()
  } catch {} finally { operating.value = false }
}

async function handleRetry() {
  if (!entity.value) return
  operating.value = true
  try {
    await taskInstanceApi.retry(entity.value.id)
    Notification.success({ content: '已提交重试' })
    startPolling()
    await fetchDetail()
  } catch {} finally { operating.value = false }
}

async function handleCancel() {
  if (!entity.value) return
  operating.value = true
  try {
    await taskInstanceApi.cancel(entity.value.id)
    Notification.success({ content: '已取消' })
    await fetchDetail()
  } catch {} finally { operating.value = false }
}

onMounted(async () => {
  loading.value = true
  try {
    await fetchDetail()
    if (entity.value && !TERMINAL_STATUSES.includes(entity.value.task_status)) {
      startPolling()
    }
  } finally { loading.value = false }
})

onUnmounted(stopPolling)
</script>
