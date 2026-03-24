<template>
  <div>
    <a-page-header title="任务实例详情" @back="$router.push('/tasks/instances')" />
    <a-card :loading="loading">
      <a-descriptions :column="2" bordered>
        <a-descriptions-item label="实例ID">{{ entity?.task_instance_id }}</a-descriptions-item>
        <a-descriptions-item label="任务名称">{{ entity?.task_name }}</a-descriptions-item>
        <a-descriptions-item label="任务类型">{{ entity?.task_type }}</a-descriptions-item>
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
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { taskInstanceApi } from '../../../api/task-instance'
import { TASK_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate, formatDuration } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import JsonViewer from '../../../components/common/json-viewer.vue'
import type { TaskInstanceEntity } from '../../../types/task'

const route = useRoute()
const loading = ref(false)
const entity = ref<TaskInstanceEntity | null>(null)

onMounted(async () => {
  loading.value = true
  try {
    const res = await taskInstanceApi.get(route.params.id as string)
    entity.value = res.data
  } catch {} finally { loading.value = false }
})
</script>
