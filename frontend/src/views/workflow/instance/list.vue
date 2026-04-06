<template>
  <div>
    <a-card title="工作流实例">
      <a-form :model="filters" layout="inline" class="filter-form">
        <a-form-item label="Meta ID">
          <a-input v-model="filters.workflow_meta_id" allow-clear placeholder="可选" style="width: 200px" />
        </a-form-item>
        <a-form-item label="版本">
          <a-input-number v-model="filters.version" :min="1" allow-clear placeholder="可选" />
        </a-form-item>
        <a-form-item label="状态">
          <a-select v-model="filters.status" allow-clear placeholder="全部" style="width: 140px">
            <a-option value="Pending">Pending</a-option>
            <a-option value="Running">Running</a-option>
            <a-option value="Await">Await</a-option>
            <a-option value="Completed">Completed</a-option>
            <a-option value="Failed">Failed</a-option>
            <a-option value="Canceled">Canceled</a-option>
            <a-option value="Suspended">Suspended</a-option>
          </a-select>
        </a-form-item>
        <a-form-item>
          <a-space>
            <a-button type="primary" @click="onSearch">查询</a-button>
            <a-button @click="onResetFilters">重置</a-button>
          </a-space>
        </a-form-item>
      </a-form>

      <a-table
        :data="list"
        :columns="columns"
        :loading="loading"
        row-key="workflow_instance_id"
        :pagination="pagination"
        @page-change="onPageChange"
        @page-size-change="onPageSizeChange"
      >
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
import { ref, reactive, onMounted } from 'vue'
import { workflowApi } from '../../../api/workflow'
import { usePermission } from '../../../composables/use-permission'
import { WORKFLOW_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { WorkflowInstanceEntity, WorkflowInstanceStatus, ListWorkflowInstancesParams } from '../../../types/workflow'

const { canExecute } = usePermission()
const list = ref<WorkflowInstanceEntity[]>([])
const loading = ref(false)

const filters = reactive<{
  workflow_meta_id: string
  version: number | undefined
  status: WorkflowInstanceStatus | undefined
}>({
  workflow_meta_id: '',
  version: undefined,
  status: undefined,
})

const pagination = reactive({
  current: 1,
  pageSize: 10,
  total: 0,
  showTotal: true,
  showPageSize: true,
  pageSizeOptions: [10, 20, 50, 100],
})

const columns = [
  { title: '实例ID', dataIndex: 'workflow_instance_id', ellipsis: true, width: 200 },
  { title: '工作流', dataIndex: 'workflow_meta_id', ellipsis: true },
  { title: '版本', dataIndex: 'workflow_version', width: 80 },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '当前节点', dataIndex: 'current_node', ellipsis: true },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 260 },
]

function buildQueryParams(): ListWorkflowInstancesParams {
  const p: ListWorkflowInstancesParams = {
    page: pagination.current,
    page_size: pagination.pageSize,
  }
  const mid = filters.workflow_meta_id.trim()
  if (mid) p.workflow_meta_id = mid
  if (filters.version != null && filters.version > 0) p.version = filters.version
  if (filters.status) p.status = filters.status
  return p
}

async function fetchList() {
  loading.value = true
  try {
    const res = await workflowApi.listInstances(buildQueryParams())
    list.value = res.data.items
    pagination.total = Number(res.data.total)
  } catch { /* interceptor */ } finally { loading.value = false }
}

function onPageChange(page: number) {
  pagination.current = page
  fetchList()
}

function onPageSizeChange(size: number) {
  pagination.pageSize = size
  pagination.current = 1
  fetchList()
}

function onSearch() {
  pagination.current = 1
  fetchList()
}

function onResetFilters() {
  filters.workflow_meta_id = ''
  filters.version = undefined
  filters.status = undefined
  pagination.current = 1
  fetchList()
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

<style scoped>
.filter-form {
  margin-bottom: 16px;
}
</style>
