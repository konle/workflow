<template>
  <div>
    <a-card title="任务实例">
      <a-form :model="filters" layout="inline" class="filter-form">
        <a-form-item label="任务ID">
          <a-input v-model="filters.task_id" allow-clear placeholder="可选" style="width: 200px" />
        </a-form-item>
        <a-form-item label="状态">
          <a-select v-model="filters.status" allow-clear placeholder="全部" style="width: 140px">
            <a-option value="Pending">Pending</a-option>
            <a-option value="Running">Running</a-option>
            <a-option value="Completed">Completed</a-option>
            <a-option value="Failed">Failed</a-option>
            <a-option value="Canceled">Canceled</a-option>
            <a-option value="Skipped">Skipped</a-option>
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
        row-key="id"
        :pagination="pagination"
        @page-change="onPageChange"
        @page-size-change="onPageSizeChange"
      >
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
            <a-button type="text" size="small" @click="$router.push(`/tasks/instances/${record.task_instance_id}`)">详情</a-button>
            <a-button v-if="record.task_status === 'Pending' && canExecute" type="text" size="small" status="success" @click="handleExecute(record.task_instance_id)">执行</a-button>
            <a-button v-if="record.task_status === 'Failed' && canExecute" type="text" size="small" @click="handleRetry(record.task_instance_id)">重试</a-button>
            <a-button v-if="['Pending','Failed'].includes(record.task_status) && canExecute" type="text" size="small" status="danger" @click="handleCancel(record.task_instance_id)">取消</a-button>
          </a-space>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { taskInstanceApi } from '../../../api/task-instance'
import { usePermission } from '../../../composables/use-permission'
import { TASK_TYPE_MAP, TASK_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate, formatDuration } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { TaskInstanceEntity, TaskInstanceStatus, ListTaskInstancesParams } from '../../../types/task'

const { canExecute } = usePermission()
const list = ref<TaskInstanceEntity[]>([])
const loading = ref(false)

const filters = reactive<{
  task_id: string
  status: TaskInstanceStatus | undefined
}>({
  task_id: '',
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
  { title: '实例ID', dataIndex: 'task_instance_id', ellipsis: true, width: 200 },
  { title: '任务名称', dataIndex: 'task_name' },
  { title: '类型', slotName: 'task_type', width: 100 },
  { title: '状态', slotName: 'task_status', width: 100 },
  { title: '耗时', slotName: 'duration', width: 100 },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 200 },
]

function buildQueryParams(): ListTaskInstancesParams {
  const p: ListTaskInstancesParams = {
    page: pagination.current,
    page_size: pagination.pageSize,
  }
  const tid = filters.task_id.trim()
  if (tid) p.task_id = tid
  if (filters.status) p.status = filters.status
  return p
}

async function fetchList() {
  loading.value = true
  try {
    const res = await taskInstanceApi.list(buildQueryParams())
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
  filters.task_id = ''
  filters.status = undefined
  pagination.current = 1
  fetchList()
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

<style scoped>
.filter-form {
  margin-bottom: 16px;
}
</style>
