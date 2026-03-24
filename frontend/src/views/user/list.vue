<template>
  <div>
    <a-card title="用户管理">
      <template #extra>
        <a-button type="primary" @click="showAssign = true">邀请用户</a-button>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="user_id">
        <template #role="{ record }">
          <a-tag>{{ record.role }}</a-tag>
        </template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="openEdit(record)">修改角色</a-button>
            <a-popconfirm content="确定移除此用户？" @ok="handleRemove(record.user_id)">
              <a-button type="text" size="small" status="danger">移除</a-button>
            </a-popconfirm>
          </a-space>
        </template>
      </a-table>
    </a-card>

    <a-modal v-model:visible="showAssign" :title="editTarget ? '修改角色' : '邀请用户'" @ok="handleSave" :ok-loading="saving">
      <a-form :model="form" layout="vertical">
        <a-form-item v-if="!editTarget" label="用户ID" required>
          <a-input v-model="form.user_id" />
        </a-form-item>
        <a-form-item label="角色" required>
          <a-select v-model="form.role">
            <a-option value="TenantAdmin">TenantAdmin</a-option>
            <a-option value="Developer">Developer</a-option>
            <a-option value="Operator">Operator</a-option>
            <a-option value="Viewer">Viewer</a-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { userApi } from '../../api/user'
import { formatDate } from '../../utils/format'
import { Notification } from '@arco-design/web-vue'
import type { UserTenantRole } from '../../types/user'

const list = ref<UserTenantRole[]>([])
const loading = ref(false)
const showAssign = ref(false)
const saving = ref(false)
const editTarget = ref<UserTenantRole | null>(null)
const form = reactive({ user_id: '', role: 'Viewer' })

const columns = [
  { title: '用户ID', dataIndex: 'user_id', ellipsis: true },
  { title: '角色', slotName: 'role', width: 140 },
  { title: '加入时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 180 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await userApi.list()
    list.value = res.data
  } catch {} finally { loading.value = false }
}

function openEdit(record: UserTenantRole) {
  editTarget.value = record
  form.user_id = record.user_id
  form.role = record.role
  showAssign.value = true
}

async function handleSave() {
  saving.value = true
  try {
    if (editTarget.value) {
      await userApi.updateRole(editTarget.value.user_id, form)
    } else {
      await userApi.assignRole(form)
    }
    Notification.success({ content: '保存成功' })
    showAssign.value = false
    editTarget.value = null
    form.user_id = ''
    form.role = 'Viewer'
    fetchList()
  } catch {} finally { saving.value = false }
}

async function handleRemove(userId: string) {
  await userApi.removeRole(userId)
  Notification.success({ content: '已移除' })
  fetchList()
}

onMounted(fetchList)
</script>
