<template>
  <div>
    <a-card title="工作流实例">
      <a-table :data="list" :columns="columns" :loading="loading" row-key="workflow_instance_id">
        <template #status="{ record }">
          <status-tag :status="record.status" :map="WORKFLOW_INSTANCE_STATUS_MAP" />
        </template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="$router.push(`/workflows/instances/${record.workflow_instance_id}`)">详情</a-button>
            <a-button v-if="record.status === 'Pending' && canExecute" type="text" size="small" status="success" @click="handleExecute(record.workflow_instance_id)">执行</a-button>
            <a-button v-if="record.status === 'Failed' && canExecute" type="text" size="small" @click="handleRetry(record.workflow_instance_id)">重试</a-button>
            <a-button v-if="record.status === 'Suspended' && canExecute" type="text" size="small" @click="handleResume(record.workflow_instance_id)">恢复</a-button>
            <a-button v-if="['Failed','Suspended'].includes(record.status) && canExecute" type="text" size="small" status="danger" @click="handleCancel(record.workflow_instance_id)">取消</a-button>
          </a-space>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { workflowApi } from '../../../api/workflow'
import { usePermission } from '../../../composables/use-permission'
import { WORKFLOW_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { WorkflowInstanceEntity } from '../../../types/workflow'

const { canExecute } = usePermission()
const list = ref<WorkflowInstanceEntity[]>([])
const loading = ref(false)

const columns = [
  { title: '实例ID', dataIndex: 'workflow_instance_id', ellipsis: true, width: 200 },
  { title: '工作流', dataIndex: 'workflow_meta_id', ellipsis: true },
  { title: '版本', dataIndex: 'workflow_version', width: 80 },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '当前节点', dataIndex: 'current_node', ellipsis: true },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 260 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await workflowApi.listInstances()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

async function handleExecute(id: string) {
  await workflowApi.executeInstance(id)
  Notification.success({ content: '已提交执行' })
  fetchList()
}

async function handleRetry(id: string) {
  await workflowApi.retryInstance(id)
  Notification.success({ content: '已提交重试' })
  fetchList()
}

async function handleResume(id: string) {
  await workflowApi.resumeInstance(id)
  Notification.success({ content: '已恢复' })
  fetchList()
}

async function handleCancel(id: string) {
  await workflowApi.cancelInstance(id)
  Notification.success({ content: '已取消' })
  fetchList()
}

onMounted(fetchList)
</script>
