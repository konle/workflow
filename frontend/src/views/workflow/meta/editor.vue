<template>
  <div>
    <a-page-header title="创建工作流" @back="$router.push('/workflows')" />
    <a-card>
      <a-form :model="form" layout="vertical" @submit-success="handleSave">
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item field="name" label="名称" :rules="[{ required: true, message: '请输入名称' }]">
              <a-input v-model="form.name" />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="status" label="状态">
              <a-select v-model="form.status">
                <a-option value="Draft">草稿</a-option>
                <a-option value="Published">已发布</a-option>
              </a-select>
            </a-form-item>
          </a-col>
        </a-row>
        <a-form-item field="description" label="描述">
          <a-textarea v-model="form.description" />
        </a-form-item>

        <a-divider>工作流表单定义</a-divider>
        <a-alert type="info" style="margin-bottom: 16px">
          此处定义的字段将在发起工作流实例时展示给用户填写，值通过 <code>{<!-- -->{key}}</code> 在节点配置中引用。表单在所有版本间共享。
        </a-alert>
        <a-form-item label="表单字段">
          <div class="form-list">
            <div v-for="(f, idx) in formFields" :key="idx" class="form-row">
              <a-input v-model="f.key" placeholder="字段名 (key)" style="width: 160px" />
              <a-input v-if="f.type === 'String'" v-model="f.value" placeholder="默认值 (可选)" style="flex: 1" />
              <a-input-number v-else-if="f.type === 'Number'" v-model="f.value" placeholder="默认值" style="flex: 1" />
              <a-select v-else-if="f.type === 'Bool'" v-model="f.value" style="flex: 1" placeholder="默认值">
                <a-option :value="true">true</a-option>
                <a-option :value="false">false</a-option>
              </a-select>
              <a-textarea v-else v-model="f.value" placeholder="JSON 默认值" :auto-size="{ minRows: 1, maxRows: 3 }" style="flex: 1" />
              <a-select v-model="f.type" style="width: 120px">
                <a-option value="String">String</a-option>
                <a-option value="Number">Number</a-option>
                <a-option value="Bool">Bool</a-option>
                <a-option value="Json">Json</a-option>
              </a-select>
              <a-input v-model="f.description" placeholder="字段说明" style="width: 200px" />
              <a-button status="danger" @click="formFields.splice(idx, 1)">
                <template #icon><icon-delete /></template>
              </a-button>
            </div>
            <a-button type="dashed" long @click="addFormField">+ 添加表单字段</a-button>
          </div>
        </a-form-item>

        <a-form-item>
          <a-space>
            <a-button type="primary" html-type="submit" :loading="saving">创建</a-button>
            <a-button @click="$router.push('/workflows')">取消</a-button>
          </a-space>
        </a-form-item>
      </a-form>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { workflowApi } from '../../../api/workflow'
import { Notification } from '@arco-design/web-vue'
import { IconDelete } from '@arco-design/web-vue/es/icon'
import type { FormField } from '../../../types/workflow'

const router = useRouter()
const saving = ref(false)

const form = reactive({
  name: '',
  description: '',
  status: 'Draft' as string,
})

interface FormRow {
  key: string
  value: any
  type: string
  description: string
}

const formFields = reactive<FormRow[]>([])

function addFormField() {
  formFields.push({ key: '', value: '', type: 'String', description: '' })
}

function toFormFields(rows: FormRow[]): FormField[] {
  return rows
    .filter(r => r.key.trim() !== '')
    .map(r => {
      const field: FormField = { key: r.key, value: r.value, type: r.type }
      if (r.description) field.description = r.description
      return field
    })
}

async function handleSave() {
  saving.value = true
  try {
    await workflowApi.createMeta({
      name: form.name,
      description: form.description,
      status: form.status as any,
      form: toFormFields(formFields),
    })
    Notification.success({ content: '创建成功' })
    router.push('/workflows')
  } catch {} finally { saving.value = false }
}
</script>

<style scoped>
.form-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}

.form-row {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}
</style>
