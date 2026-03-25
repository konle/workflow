<template>
  <div>
    <a-page-header :title="meta?.name || '工作流详情'" @back="$router.push('/workflows')" />

    <a-tabs default-active-key="info">
      <a-tab-pane key="info" title="基础信息">
        <a-card :loading="loading">
          <a-form v-if="meta" :model="meta" layout="vertical">
            <a-row :gutter="16">
              <a-col :span="12">
                <a-form-item label="名称"><a-input v-model="meta.name" :disabled="!canWrite" /></a-form-item>
              </a-col>
              <a-col :span="12">
                <a-form-item label="状态">
                  <a-select v-model="meta.status" :disabled="!canWrite">
                    <a-option value="Draft">草稿</a-option>
                    <a-option value="Published">已发布</a-option>
                  </a-select>
                </a-form-item>
              </a-col>
            </a-row>
            <a-form-item label="描述"><a-textarea v-model="meta.description" :disabled="!canWrite" /></a-form-item>

            <a-divider>工作流表单定义</a-divider>
            <a-alert type="info" style="margin-bottom: 16px">
              此处定义的字段将在发起工作流实例时展示给用户填写，值通过 <code>{<!-- -->{key}}</code> 在节点配置中引用。表单在所有版本间共享。
            </a-alert>
            <a-form-item label="表单字段">
              <div class="form-list">
                <div v-for="(f, idx) in formFields" :key="idx" class="form-row">
                  <a-input v-model="f.key" placeholder="字段名 (key)" style="width: 160px" :disabled="!canWrite" />
                  <a-input v-if="f.type === 'String'" v-model="f.value" placeholder="默认值 (可选)" style="flex: 1" :disabled="!canWrite" />
                  <a-input-number v-else-if="f.type === 'Number'" v-model="f.value" placeholder="默认值" style="flex: 1" :disabled="!canWrite" />
                  <a-select v-else-if="f.type === 'Bool'" v-model="f.value" style="flex: 1" placeholder="默认值" :disabled="!canWrite">
                    <a-option :value="true">true</a-option>
                    <a-option :value="false">false</a-option>
                  </a-select>
                  <a-textarea v-else v-model="f.value" placeholder="JSON 默认值" :auto-size="{ minRows: 1, maxRows: 3 }" style="flex: 1" :disabled="!canWrite" />
                  <a-select v-model="f.type" style="width: 120px" :disabled="!canWrite">
                    <a-option value="String">String</a-option>
                    <a-option value="Number">Number</a-option>
                    <a-option value="Bool">Bool</a-option>
                    <a-option value="Json">Json</a-option>
                  </a-select>
                  <a-input v-model="f.description" placeholder="字段说明" style="width: 200px" :disabled="!canWrite" />
                  <a-button v-if="canWrite" status="danger" @click="formFields.splice(idx, 1)">
                    <template #icon><icon-delete /></template>
                  </a-button>
                </div>
                <a-button v-if="canWrite" type="dashed" long @click="addFormField">+ 添加表单字段</a-button>
              </div>
            </a-form-item>

            <a-form-item v-if="canWrite">
              <a-button type="primary" @click="handleSaveMeta" :loading="savingMeta">保存</a-button>
            </a-form-item>
          </a-form>
        </a-card>
      </a-tab-pane>

      <a-tab-pane key="versions" title="版本管理">
        <a-card>
          <template #extra>
            <a-button v-if="canWrite" type="primary" size="small" @click="$router.push(`/workflows/${metaId}/editor`)">新建版本</a-button>
          </template>
          <a-list :data="versions" :loading="versionsLoading">
            <template #item="{ item }">
              <a-list-item>
                <a-list-item-meta>
                  <template #title>版本 {{ item.version }}</template>
                  <template #description>
                    {{ item.nodes?.length || 0 }} 个节点 · {{ formatDate(item.created_at) }}
                  </template>
                </a-list-item-meta>
                <template #actions>
                  <a-button type="text" size="small" @click="$router.push(`/workflows/${metaId}/editor/${item.version}`)">编辑</a-button>
                  <a-button type="text" size="small" status="success" @click="openLaunch(item.version)">发起实例</a-button>
                  <a-popconfirm v-if="canWrite" content="确定删除此版本？" @ok="handleDeleteVersion(item.version)">
                    <a-button type="text" size="small" status="danger">删除</a-button>
                  </a-popconfirm>
                </template>
              </a-list-item>
            </template>
          </a-list>
        </a-card>

        <a-modal v-model:visible="showLaunch" title="发起工作流实例" @ok="handleLaunch" :ok-loading="launching">
          <a-alert style="margin-bottom: 12px">版本: {{ launchVersion }}</a-alert>
          <a-form :model="launchForm" layout="vertical">
            <template v-for="field in (meta?.form || [])" :key="field.key">
              <a-form-item :label="field.description || field.key">
                <a-switch v-if="field.type === 'Bool'" v-model="launchCtx[field.key]" />
                <a-input-number v-else-if="field.type === 'Number'" v-model="launchCtx[field.key]" />
                <a-textarea v-else-if="field.type === 'Json'" v-model="launchCtx[field.key]" :auto-size="{ minRows: 2 }" />
                <a-input v-else v-model="launchCtx[field.key]" />
              </a-form-item>
            </template>
            <a-form-item v-if="!meta?.form?.length" label="Context (JSON)">
              <a-textarea v-model="launchForm.contextJson" :auto-size="{ minRows: 3 }" placeholder="{}" />
            </a-form-item>
          </a-form>
          <a-checkbox v-model="launchForm.autoExecute">创建后立即执行</a-checkbox>
        </a-modal>
      </a-tab-pane>

      <a-tab-pane key="variables" title="模板变量">
        <meta-variable-list v-if="metaId" :meta-id="metaId" />
      </a-tab-pane>
    </a-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { workflowApi } from '../../../api/workflow'
import { usePermission } from '../../../composables/use-permission'
import { formatDate } from '../../../utils/format'
import { Notification } from '@arco-design/web-vue'
import { IconDelete } from '@arco-design/web-vue/es/icon'
import MetaVariableList from '../../variable/meta-list.vue'
import type { WorkflowMetaEntity, WorkflowEntity, FormField } from '../../../types/workflow'

const route = useRoute()
const router = useRouter()
const { canWrite } = usePermission()
const metaId = route.params.metaId as string

const meta = ref<WorkflowMetaEntity | null>(null)
const loading = ref(false)
const savingMeta = ref(false)
const versions = ref<WorkflowEntity[]>([])
const versionsLoading = ref(false)

const showLaunch = ref(false)
const launchVersion = ref(0)
const launching = ref(false)
const launchForm = reactive({ contextJson: '{}', autoExecute: true })
const launchCtx = reactive<Record<string, any>>({})

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

function loadFormFields(fields: FormField[] | undefined | null) {
  formFields.splice(0, formFields.length)
  if (!fields || !Array.isArray(fields)) return
  for (const f of fields) {
    formFields.push({
      key: f.key,
      value: f.value,
      type: f.type || 'String',
      description: f.description || '',
    })
  }
}

async function fetchMeta() {
  loading.value = true
  try {
    const res = await workflowApi.getMeta(metaId)
    meta.value = res.data
    loadFormFields(res.data.form)
  } catch {} finally { loading.value = false }
}

async function fetchVersions() {
  versionsLoading.value = true
  const found: WorkflowEntity[] = []
  for (let v = 1; v <= 20; v++) {
    try {
      const res = await workflowApi.getTemplate(metaId, v)
      found.push(res.data)
    } catch { break }
  }
  versions.value = found
  versionsLoading.value = false
}

async function handleSaveMeta() {
  if (!meta.value) return
  savingMeta.value = true
  try {
    await workflowApi.updateMeta(metaId, {
      name: meta.value.name,
      description: meta.value.description,
      status: meta.value.status,
      form: toFormFields(formFields),
    })
    Notification.success({ content: '保存成功' })
  } catch {} finally { savingMeta.value = false }
}

async function handleDeleteVersion(version: number) {
  await workflowApi.deleteTemplate(metaId, version)
  Notification.success({ content: '版本已删除' })
  fetchVersions()
}

function openLaunch(version: number) {
  launchVersion.value = version
  launchForm.contextJson = '{}'
  launchForm.autoExecute = true
  if (meta.value?.form) {
    for (const f of meta.value.form) {
      launchCtx[f.key] = f.value ?? ''
    }
  }
  showLaunch.value = true
}

async function handleLaunch() {
  launching.value = true
  try {
    let context: any = {}
    if (meta.value?.form?.length) {
      context = { ...launchCtx }
    } else {
      try { context = JSON.parse(launchForm.contextJson) } catch { context = {} }
    }
    const res = await workflowApi.createInstance({
      workflow_meta_id: metaId,
      version: launchVersion.value,
      context,
    })
    if (launchForm.autoExecute) {
      await workflowApi.executeInstance(res.data.workflow_instance_id)
      Notification.success({ content: '实例已创建并执行' })
    } else {
      Notification.success({ content: '实例已创建' })
    }
    showLaunch.value = false
    router.push('/workflows/instances')
  } catch {} finally { launching.value = false }
}

onMounted(() => { fetchMeta(); fetchVersions() })
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
