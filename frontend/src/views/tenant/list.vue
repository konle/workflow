<template>
  <div>
    <a-card title="租户管理">
      <template #extra>
        <a-button type="primary" @click="showCreate = true">创建租户</a-button>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="tenant_id">
        <template #status="{ record }">
          <status-tag :status="record.status" :map="TENANT_STATUS_MAP" />
        </template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="openEdit(record)">编辑</a-button>
            <a-popconfirm content="确定暂停此租户？" @ok="handleSuspend(record.tenant_id)">
              <a-button type="text" size="small" status="warning" :disabled="record.status !== 'Active'">暂停</a-button>
            </a-popconfirm>
            <a-popconfirm content="确定删除此租户？" @ok="handleDelete(record.tenant_id)">
              <a-button type="text" size="small" status="danger">删除</a-button>
            </a-popconfirm>
          </a-space>
        </template>
      </a-table>
    </a-card>

    <a-modal v-model:visible="showCreate" :title="editTarget ? '编辑租户' : '创建租户'" @ok="handleSave" :ok-loading="saving">
      <a-form :model="form" layout="vertical">
        <a-form-item label="名称" required><a-input v-model="form.name" /></a-form-item>
        <a-form-item label="描述"><a-textarea v-model="form.description" /></a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { tenantApi } from '../../api/tenant'
import { TENANT_STATUS_MAP } from '../../utils/constants'
import { formatDate } from '../../utils/format'
import StatusTag from '../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { TenantEntity } from '../../types/tenant'

const list = ref<TenantEntity[]>([])
const loading = ref(false)
const showCreate = ref(false)
const saving = ref(false)
const editTarget = ref<TenantEntity | null>(null)
const form = reactive({ name: '', description: '' })

const columns = [
  { title: '名称', dataIndex: 'name' },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '最大工作流', dataIndex: 'max_workflows', width: 120 },
  { title: '最大实例', dataIndex: 'max_instances', width: 120 },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 200 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await tenantApi.list()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

function openEdit(record: TenantEntity) {
  editTarget.value = record
  form.name = record.name
  form.description = record.description
  showCreate.value = true
}

async function handleSave() {
  saving.value = true
  try {
    if (editTarget.value) {
      await tenantApi.update(editTarget.value.tenant_id, form)
    } else {
      await tenantApi.create(form)
    }
    Notification.success({ content: '保存成功' })
    showCreate.value = false
    editTarget.value = null
    form.name = ''
    form.description = ''
    fetchList()
  } catch {} finally { saving.value = false }
}

async function handleSuspend(id: string) {
  await tenantApi.suspend(id)
  Notification.success({ content: '已暂停' })
  fetchList()
}

async function handleDelete(id: string) {
  await tenantApi.delete(id)
  Notification.success({ content: '已删除' })
  fetchList()
}

onMounted(fetchList)
</script>
