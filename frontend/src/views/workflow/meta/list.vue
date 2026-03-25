<template>
  <div>
    <a-card title="工作流管理">
      <template #extra>
        <a-button v-if="canWrite" type="primary" @click="$router.push('/workflows/create')">创建工作流</a-button>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="workflow_meta_id">
        <template #status="{ record }">
          <status-tag :status="record.status" :map="TEMPLATE_STATUS_MAP" />
        </template>
        <template #form_count="{ record }">{{ record.form?.length || 0 }}</template>
        <template #updated_at="{ record }">{{ formatDate(record.updated_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="$router.push(`/workflows/${record.workflow_meta_id}`)">详情</a-button>
            <a-button v-if="canWrite" type="text" size="small" @click="$router.push(`/workflows/${record.workflow_meta_id}/editor`)">新建版本</a-button>
            <a-popconfirm v-if="canWrite" content="确定删除？" @ok="handleDelete(record.workflow_meta_id)">
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
import { workflowApi } from '../../../api/workflow'
import { usePermission } from '../../../composables/use-permission'
import { TEMPLATE_STATUS_MAP } from '../../../utils/constants'
import { formatDate } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { WorkflowMetaEntity } from '../../../types/workflow'

const { canWrite } = usePermission()
const list = ref<WorkflowMetaEntity[]>([])
const loading = ref(false)

const columns = [
  { title: '名称', dataIndex: 'name' },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '表单字段数', slotName: 'form_count', width: 120 },
  { title: '更新时间', slotName: 'updated_at', width: 180 },
  { title: '操作', slotName: 'action', width: 240 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await workflowApi.listMeta()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

async function handleDelete(id: string) {
  await workflowApi.deleteMeta(id)
  Notification.success({ content: '已删除' })
  fetchList()
}

onMounted(fetchList)
</script>
