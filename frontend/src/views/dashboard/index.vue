<template>
  <div class="dashboard">
    <a-row :gutter="16">
      <a-col :span="6">
        <a-card>
          <a-statistic title="任务模板" :value="stats.taskCount" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="工作流模板" :value="stats.workflowCount" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="运行中实例" :value="stats.runningCount" value-style="color: #3491FA" />
        </a-card>
      </a-col>
      <a-col :span="6">
        <a-card>
          <a-statistic title="失败实例" :value="stats.failedCount" value-style="color: #F53F3F" />
        </a-card>
      </a-col>
    </a-row>

    <a-card title="最近失败的工作流实例" style="margin-top: 16px">
      <a-table :data="failedInstances" :columns="columns" :pagination="false" size="small">
        <template #status="{ record }">
          <status-tag :status="record.status" :map="WORKFLOW_INSTANCE_STATUS_MAP" />
        </template>
        <template #created_at="{ record }">
          {{ formatDate(record.created_at) }}
        </template>
        <template #action="{ record }">
          <a-button type="text" size="small" @click="$router.push(`/workflows/instances/${record.workflow_instance_id}`)">
            查看
          </a-button>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { taskApi } from '../../api/task'
import { workflowApi } from '../../api/workflow'
import { WORKFLOW_INSTANCE_STATUS_MAP } from '../../utils/constants'
import { formatDate } from '../../utils/format'
import StatusTag from '../../components/common/status-tag.vue'
import type { WorkflowInstanceEntity } from '../../types/workflow'

const stats = reactive({ taskCount: 0, workflowCount: 0, runningCount: 0, failedCount: 0 })
const failedInstances = ref<WorkflowInstanceEntity[]>([])

const columns = [
  { title: '实例ID', dataIndex: 'workflow_instance_id', ellipsis: true },
  { title: '工作流', dataIndex: 'workflow_meta_id', ellipsis: true },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 80 },
]

onMounted(async () => {
  try {
    const [tasks, metas, instances] = await Promise.all([
      taskApi.list(),
      workflowApi.listMeta(),
      workflowApi.listInstances(),
    ])
    stats.taskCount = tasks.data.length
    stats.workflowCount = metas.data.length
    stats.runningCount = instances.data.filter(i => i.status === 'Running').length
    stats.failedCount = instances.data.filter(i => i.status === 'Failed').length
    failedInstances.value = instances.data.filter(i => i.status === 'Failed').slice(0, 5)
  } catch { /* handled by interceptor */ }
})
</script>

<style scoped>
.dashboard :deep(.arco-card) {
  border-radius: 8px;
}
</style>
