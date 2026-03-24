<template>
  <div>
    <a-card title="任务实例">
      <a-table :data="list" :columns="columns" :loading="loading" row-key="id">
        <template #task_type="{ record }">
          <a-tag :color="TASK_TYPE_MAP[record.task_type]?.color">{{ TASK_TYPE_MAP[record.task_type]?.label || record.task_type }}</a-tag>
        </template>
        <template #task_status="{ record }">
          <status-tag :status="record.task_status" :map="TASK_INSTANCE_STATUS_MAP" />
        </template>
        <template #duration="{ record }">{{ formatDuration(record.execution_duration) }}</template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="$router.push(`/tasks/instances/${record.id}`)">详情</a-button>
            <a-button v-if="record.task_status === 'Pending' && canExecute" type="text" size="small" status="success" @click="handleExecute(record.id)">执行</a-button>
            <a-button v-if="record.task_status === 'Failed' && canExecute" type="text" size="small" @click="handleRetry(record.id)">重试</a-button>
            <a-button v-if="['Pending','Failed'].includes(record.task_status) && canExecute" type="text" size="small" status="danger" @click="handleCancel(record.id)">取消</a-button>
          </a-space>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { taskInstanceApi } from '../../../api/task-instance'
import { usePermission } from '../../../composables/use-permission'
import { TASK_TYPE_MAP, TASK_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate, formatDuration } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { TaskInstanceEntity } from '../../../types/task'

const { canExecute } = usePermission()
const list = ref<TaskInstanceEntity[]>([])
const loading = ref(false)

const columns = [
  { title: '实例ID', dataIndex: 'task_instance_id', ellipsis: true, width: 200 },
  { title: '任务名称', dataIndex: 'task_name' },
  { title: '类型', slotName: 'task_type', width: 100 },
  { title: '状态', slotName: 'task_status', width: 100 },
  { title: '耗时', slotName: 'duration', width: 100 },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 200 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await taskInstanceApi.list()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

async function handleExecute(id: string) {
  await taskInstanceApi.execute(id)
  Notification.success({ content: '已提交执行' })
  fetchList()
}

async function handleRetry(id: string) {
  await taskInstanceApi.retry(id)
  Notification.success({ content: '已提交重试' })
  fetchList()
}

async function handleCancel(id: string) {
  await taskInstanceApi.cancel(id)
  Notification.success({ content: '已取消' })
  fetchList()
}

onMounted(fetchList)
</script>
