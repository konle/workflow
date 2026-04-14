<template>
  <div>
    <a-card title="用户管理">
      <template #extra>
        <a-space>
          <a-button type="primary" @click="showCreate = true">创建用户</a-button>
          <a-button @click="showAssign = true">邀请用户</a-button>
        </a-space>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="user_id">
        <template #role="{ record }">
          <a-tag>{{ record.role }}</a-tag>
        </template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="small" @click="openEdit(record)">修改角色</a-button>
            <a-popconfirm content="确定移除此用户？" @ok="handleRemove(record.username)">
              <a-button type="text" size="small" status="danger">移除</a-button>
            </a-popconfirm>
          </a-space>
        </template>
      </a-table>
    </a-card>

    <!-- 邀请已有用户 -->
    <a-modal v-model:visible="showAssign" :title="editTarget ? '修改角色' : '邀请用户'" @ok="handleSave" :ok-loading="saving">
      <a-form :model="assignForm" layout="vertical">
        <a-form-item v-if="!editTarget" label="用户名" required>
          <a-input v-model="assignForm.username" placeholder="输入已注册的用户名" />
        </a-form-item>
        <a-form-item label="角色" required>
          <a-select v-model="assignForm.role">
            <a-option value="TenantAdmin">TenantAdmin</a-option>
            <a-option value="Developer">Developer</a-option>
            <a-option value="Operator">Operator</a-option>
            <a-option value="Viewer">Viewer</a-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 创建新用户 -->
    <a-modal v-model:visible="showCreate" title="创建用户" @ok="handleCreate" :ok-loading="creating">
      <a-form :model="createForm" layout="vertical">
        <a-form-item label="用户名" required>
          <a-input v-model="createForm.username" placeholder="新用户的用户名" />
        </a-form-item>
        <a-form-item label="邮箱" required>
          <a-input v-model="createForm.email" placeholder="用户邮箱" />
        </a-form-item>
        <a-form-item label="角色" required>
          <a-select v-model="createForm.role">
            <a-option value="TenantAdmin">TenantAdmin</a-option>
            <a-option value="Developer">Developer</a-option>
            <a-option value="Operator">Operator</a-option>
            <a-option value="Viewer">Viewer</a-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 初始密码展示 -->
    <a-modal v-model:visible="showPassword" title="用户创建成功" :footer="false" :mask-closable="false">
      <a-result status="success" :title="`用户 ${createdUsername} 创建成功`">
        <template #subtitle>
          <div style="margin-top: 8px">
            <p>初始密码（仅显示一次，请妥善保存）：</p>
            <a-typography-paragraph copyable style="font-size: 18px; font-weight: 600; margin-top: 8px">
              {{ createdPassword }}
            </a-typography-paragraph>
          </div>
        </template>
        <template #extra>
          <a-button type="primary" @click="showPassword = false">我已记录密码</a-button>
        </template>
      </a-result>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { userApi } from '../../api/user'
import { formatDate } from '../../utils/format'
import { Notification } from '@arco-design/web-vue'
import type { UserRoleInfo } from '../../types/user'

const list = ref<UserRoleInfo[]>([])
const loading = ref(false)
const showAssign = ref(false)
const showCreate = ref(false)
const showPassword = ref(false)
const saving = ref(false)
const creating = ref(false)
const editTarget = ref<UserRoleInfo | null>(null)
const assignForm = reactive({ username: '', role: 'Viewer' })
const createForm = reactive({ username: '', email: '', role: 'Viewer' })
const createdUsername = ref('')
const createdPassword = ref('')

const columns = [
  { title: '用户名', dataIndex: 'username' },
  { title: '邮箱', dataIndex: 'email', ellipsis: true },
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

function openEdit(record: UserRoleInfo) {
  editTarget.value = record
  assignForm.username = record.username
  assignForm.role = record.role
  showAssign.value = true
}

async function handleSave() {
  saving.value = true
  try {
    if (editTarget.value) {
      await userApi.updateRole(editTarget.value.username, assignForm)
    } else {
      await userApi.assignRole(assignForm)
    }
    Notification.success({ content: '保存成功' })
    showAssign.value = false
    editTarget.value = null
    assignForm.username = ''
    assignForm.role = 'Viewer'
    fetchList()
  } catch {} finally { saving.value = false }
}

async function handleCreate() {
  creating.value = true
  try {
    const res = await userApi.createUser(createForm)
    createdUsername.value = res.data.username
    createdPassword.value = res.data.initial_password
    showCreate.value = false
    showPassword.value = true
    createForm.username = ''
    createForm.email = ''
    createForm.role = 'Viewer'
    fetchList()
  } catch {} finally { creating.value = false }
}

async function handleRemove(username: string) {
  await userApi.removeRole(username)
  Notification.success({ content: '已移除' })
  fetchList()
}

onMounted(fetchList)
</script>
