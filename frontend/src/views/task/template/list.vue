<template>
  <div>
    <a-card title="任务模板">
      <template #extra>
        <a-button v-if="canWrite" type="primary" @click="$router.push('/tasks/create')">创建任务</a-button>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="id">
        <template #task_type="{ record }">
          <a-tag :color="TASK_TYPE_MAP[record.task_type]?.color">{{ TASK_TYPE_MAP[record.task_type]?.label || record.task_type }}</a-tag>
        </template>
        <template #status="{ record }">
          <status-tag :status="record.status" :map="TEMPLATE_STATUS_MAP" />
        </template>
        <template #updated_at="{ record }">{{ formatDate(record.updated_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button v-if="canWrite" type="text" size="small" @click="$router.push(`/tasks/${record.id}/edit`)">编辑</a-button>
            <a-popconfirm v-if="canWrite" content="确定删除？" @ok="handleDelete(record.id)">
              <a-button type="text" size="small" status="danger">删除</a-button>
            </a-popconfirm>
          </a-space>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { taskApi } from '../../../api/task'
import { usePermission } from '../../../composables/use-permission'
import { TASK_TYPE_MAP, TEMPLATE_STATUS_MAP } from '../../../utils/constants'
import { formatDate } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { TaskEntity } from '../../../types/task'

const { canWrite } = usePermission()
const list = ref<TaskEntity[]>([])
const loading = ref(false)

const columns = [
  { title: '名称', dataIndex: 'name' },
  { title: '类型', slotName: 'task_type', width: 120 },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '描述', dataIndex: 'description', ellipsis: true },
  { title: '更新时间', slotName: 'updated_at', width: 180 },
  { title: '操作', slotName: 'action', width: 140 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await taskApi.list()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

async function handleDelete(id: string) {
  await taskApi.delete(id)
  Notification.success({ content: '已删除' })
  fetchList()
}

onMounted(fetchList)
</script>
