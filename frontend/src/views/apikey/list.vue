<template>
  <div>
    <a-card title="API Keys">
      <template #extra>
        <a-button type="primary" @click="showCreate = true">创建 API Key</a-button>
      </template>
      <a-table :data="list" :columns="columns" :loading="loading" row-key="id">
        <template #key_prefix="{ record }">
          <a-typography-text code>{{ record.key_prefix }}****</a-typography-text>
        </template>
        <template #role="{ record }">
          <a-tag>{{ record.role }}</a-tag>
        </template>
        <template #status="{ record }">
          <a-tag :color="record.status === 'Active' ? 'green' : 'red'">
            {{ record.status === 'Active' ? '正常' : '已吊销' }}
          </a-tag>
        </template>
        <template #expires_at="{ record }">
          {{ record.expires_at ? formatDate(record.expires_at) : '永不过期' }}
        </template>
        <template #last_used_at="{ record }">
          {{ record.last_used_at ? formatDate(record.last_used_at) : '-' }}
        </template>
        <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
        <template #action="{ record }">
          <a-popconfirm
            v-if="record.status === 'Active'"
            content="吊销后该 Key 将无法再换取 Token，且已签发的 Token 在到期前仍有效。确定吊销？"
            @ok="handleRevoke(record.id)"
          >
            <a-button type="text" size="small" status="danger">吊销</a-button>
          </a-popconfirm>
        </template>
      </a-table>
    </a-card>

    <a-modal v-model:visible="showCreate" title="创建 API Key" @ok="handleCreate" :ok-loading="creating">
      <a-form :model="createForm" layout="vertical">
        <a-form-item label="名称" required>
          <a-input v-model="createForm.name" placeholder="如 CI/CD Pipeline" />
        </a-form-item>
        <a-form-item label="角色" required>
          <a-select v-model="createForm.role" placeholder="选择角色">
            <a-option value="Developer">Developer（模板读写 + 实例执行）</a-option>
            <a-option value="Operator">Operator（实例执行）</a-option>
            <a-option value="Viewer">Viewer（只读）</a-option>
          </a-select>
        </a-form-item>
        <a-form-item label="有效期">
          <a-date-picker
            v-model="createForm.expires_at"
            style="width: 100%"
            placeholder="留空表示永不过期"
            show-time
          />
        </a-form-item>
        <a-form-item label="Token 有效期（秒）">
          <a-input-number v-model="createForm.token_ttl_secs" :min="60" :max="86400" :default-value="3600" style="width: 100%" />
          <template #extra>签发的 JWT Token 有效时长，默认 3600 秒（1 小时），上限 86400 秒（24 小时）</template>
        </a-form-item>
      </a-form>
    </a-modal>

    <a-modal v-model:visible="showKeyResult" title="API Key 已创建" :footer="false" :mask-closable="false">
      <a-result status="success" title="创建成功">
        <template #subtitle>
          <div style="text-align: left; margin-top: 12px">
            <a-alert type="warning" style="margin-bottom: 12px">
              请立即复制并妥善保存此 Key。关闭弹窗后将无法再次查看完整 Key。
            </a-alert>
            <a-descriptions :column="1" size="small" bordered>
              <a-descriptions-item label="名称">{{ keyResult?.name }}</a-descriptions-item>
              <a-descriptions-item label="角色">{{ keyResult?.role }}</a-descriptions-item>
              <a-descriptions-item label="API Key">
                <a-typography-paragraph copyable :copy-text="keyResult?.key" style="margin: 0">
                  <code>{{ keyResult?.key }}</code>
                </a-typography-paragraph>
              </a-descriptions-item>
            </a-descriptions>
            <div style="text-align: center; margin-top: 16px">
              <a-button type="primary" @click="showKeyResult = false">我已保存，关闭</a-button>
            </div>
          </div>
        </template>
      </a-result>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiKeyApi } from '../../api/apikey'
import { formatDate } from '../../utils/format'
import { Notification } from '@arco-design/web-vue'
import type { ApiKeyListItem, CreateApiKeyResponse } from '../../types/apikey'

const list = ref<ApiKeyListItem[]>([])
const loading = ref(false)
const showCreate = ref(false)
const creating = ref(false)
const showKeyResult = ref(false)
const keyResult = ref<CreateApiKeyResponse | null>(null)

const createForm = ref({
  name: '',
  role: 'Developer',
  expires_at: '' as string | undefined,
  token_ttl_secs: 3600,
})

const columns = [
  { title: '名称', dataIndex: 'name' },
  { title: 'Key 前缀', slotName: 'key_prefix', width: 140 },
  { title: '角色', slotName: 'role', width: 120 },
  { title: '状态', slotName: 'status', width: 90 },
  { title: '有效期', slotName: 'expires_at', width: 170 },
  { title: 'Token TTL', dataIndex: 'token_ttl_secs', width: 100 },
  { title: '最近使用', slotName: 'last_used_at', width: 170 },
  { title: '创建时间', slotName: 'created_at', width: 170 },
  { title: '操作', slotName: 'action', width: 80 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await apiKeyApi.list()
    list.value = res.data
  } catch {}
  loading.value = false
}

async function handleCreate() {
  if (!createForm.value.name || !createForm.value.role) {
    Notification.warning({ content: '请填写名称和角色' })
    return
  }
  creating.value = true
  try {
    const payload: any = {
      name: createForm.value.name,
      role: createForm.value.role,
      token_ttl_secs: createForm.value.token_ttl_secs || 3600,
    }
    if (createForm.value.expires_at) {
      payload.expires_at = new Date(createForm.value.expires_at).toISOString()
    }
    const res = await apiKeyApi.create(payload)
    keyResult.value = res.data
    showCreate.value = false
    showKeyResult.value = true
    createForm.value = { name: '', role: 'Developer', expires_at: undefined, token_ttl_secs: 3600 }
    fetchList()
  } catch (e: any) {
    Notification.error({ content: e?.message || '创建失败' })
  }
  creating.value = false
}

async function handleRevoke(id: string) {
  try {
    await apiKeyApi.revoke(id)
    Notification.success({ content: '已吊销' })
    fetchList()
  } catch (e: any) {
    Notification.error({ content: e?.message || '吊销失败' })
  }
}

onMounted(fetchList)
</script>
