<template>
  <div>
    <a-page-header :title="isEdit ? '编辑任务模板' : '创建任务模板'" @back="$router.push('/tasks')" />
    <a-card :loading="pageLoading">
      <a-form :model="form" layout="vertical" @submit-success="handleSave">
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item field="name" label="名称" :rules="[{ required: true, message: '请输入名称' }]">
              <a-input v-model="form.name" />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="task_type" label="任务类型" :rules="[{ required: true, message: '请选择类型' }]">
              <a-select v-model="form.task_type" :disabled="isEdit">
                <a-option value="Http">HTTP</a-option>
                <a-option value="Grpc">gRPC</a-option>
                <a-option value="Approval">审批</a-option>
              </a-select>
            </a-form-item>
          </a-col>
        </a-row>
        <a-form-item field="description" label="描述">
          <a-textarea v-model="form.description" />
        </a-form-item>
        <a-form-item field="status" label="状态">
          <a-radio-group v-model="form.status">
            <a-radio value="Draft">草稿</a-radio>
            <a-radio value="Published">已发布</a-radio>
          </a-radio-group>
        </a-form-item>

        <a-divider>任务配置</a-divider>

        <template v-if="form.task_type === 'Http'">
          <a-row :gutter="16">
            <a-col :span="16">
              <a-form-item label="URL" required>
                <a-input v-model="httpConfig.url" placeholder="https://example.com/api  支持 {{变量}} 模板" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="Method" required>
                <a-select v-model="httpConfig.method">
                  <a-option value="Get">GET</a-option>
                  <a-option value="Post">POST</a-option>
                  <a-option value="Put">PUT</a-option>
                  <a-option value="Delete">DELETE</a-option>
                  <a-option value="Head">HEAD</a-option>
                </a-select>
              </a-form-item>
            </a-col>
          </a-row>
          <a-form-item label="Headers (JSON)">
            <a-textarea v-model="headersJson" :auto-size="{ minRows: 2 }" placeholder='{"Content-Type": "application/json"}' />
          </a-form-item>
          <a-form-item label="Body (JSON)">
            <a-textarea v-model="bodyJson" :auto-size="{ minRows: 3 }" />
          </a-form-item>
          <a-row :gutter="16">
            <a-col :span="8">
              <a-form-item label="超时 (秒)">
                <a-input-number v-model="httpConfig.timeout" :min="1" :max="600" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="重试次数">
                <a-input-number v-model="httpConfig.retry_count" :min="0" :max="10" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="重试延迟 (秒)">
                <a-input-number v-model="httpConfig.retry_delay" :min="0" />
              </a-form-item>
            </a-col>
          </a-row>
          <a-form-item label="成功条件">
            <a-input v-model="httpConfig.success_condition" placeholder="可选，如 status == 200" />
          </a-form-item>
        </template>

        <template v-else-if="form.task_type === 'Grpc' || form.task_type === 'Approval'">
          <a-alert type="info">{{ form.task_type }} 类型配置即将支持</a-alert>
        </template>

        <a-form-item>
          <a-space>
            <a-button type="primary" html-type="submit" :loading="saving">保存</a-button>
            <a-button @click="$router.push('/tasks')">取消</a-button>
          </a-space>
        </a-form-item>
      </a-form>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { taskApi } from '../../../api/task'
import { Notification } from '@arco-design/web-vue'
import type { TaskEntity, HttpMethod } from '../../../types/task'

const route = useRoute()
const router = useRouter()
const isEdit = computed(() => !!route.params.id)
const pageLoading = ref(false)
const saving = ref(false)

const form = reactive({
  name: '',
  task_type: 'Http' as string,
  description: '',
  status: 'Draft' as string,
})

const httpConfig = reactive({
  url: '',
  method: 'Get' as HttpMethod,
  headers: {} as Record<string, string>,
  body: null as any,
  form: null as any,
  retry_count: 0,
  retry_delay: 0,
  timeout: 30,
  success_condition: null as string | null,
})

const headersJson = ref('{}')
const bodyJson = ref('')

function buildTaskTemplate() {
  if (form.task_type === 'Http') {
    try { httpConfig.headers = JSON.parse(headersJson.value || '{}') } catch { httpConfig.headers = {} }
    const bodyVal = bodyJson.value.trim()
    const bodyField = bodyVal ? { key: 'body', value: bodyVal, type: 'json' } : null
    return {
      Http: {
        ...httpConfig,
        body: bodyField,
        form: null,
        success_condition: httpConfig.success_condition || null,
      },
    }
  }
  return form.task_type
}

async function handleSave() {
  saving.value = true
  try {
    const payload: Partial<TaskEntity> = {
      name: form.name,
      task_type: form.task_type as any,
      description: form.description,
      status: form.status as any,
      task_template: buildTaskTemplate() as any,
    }
    if (isEdit.value) {
      await taskApi.update(route.params.id as string, payload)
    } else {
      await taskApi.create(payload)
    }
    Notification.success({ content: '保存成功' })
    router.push('/tasks')
  } catch {} finally { saving.value = false }
}

onMounted(async () => {
  if (isEdit.value) {
    pageLoading.value = true
    try {
      const res = await taskApi.get(route.params.id as string)
      const entity = res.data
      form.name = entity.name
      form.task_type = entity.task_type
      form.description = entity.description
      form.status = entity.status
      if (entity.task_type === 'Http' && typeof entity.task_template === 'object' && 'Http' in entity.task_template) {
        const tpl = entity.task_template.Http
        Object.assign(httpConfig, tpl)
        headersJson.value = JSON.stringify(tpl.headers || {}, null, 2)
        bodyJson.value = tpl.body ? (typeof tpl.body.value === 'string' ? tpl.body.value : JSON.stringify(tpl.body.value)) : ''
      }
    } catch {} finally { pageLoading.value = false }
  }
})
</script>
