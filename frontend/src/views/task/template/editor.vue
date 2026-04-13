<template>
  <div>
    <a-page-header :title="isEdit ? '编辑任务模板' : '创建任务模板'" @back="$router.push('/tasks')" />

    <a-tabs default-active-key="config" @change="handleTabChange">
      <a-tab-pane key="config" title="模板配置">
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

            <!-- ==================== HTTP 配置 ==================== -->
            <template v-if="form.task_type === 'Http'">
              <a-divider>请求配置</a-divider>

              <!-- URL & Method -->
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

              <!-- Headers -->
              <a-form-item label="Headers">
                <div class="form-list">
                  <div class="preset-tags">
                    <span class="preset-label">常用：</span>
                    <a-tag v-for="p in headerPresets" :key="p.key" color="arcoblue" class="preset-tag" @click="addPresetHeader(p)">
                      {{ p.key }}
                    </a-tag>
                  </div>
                  <div v-for="(h, idx) in httpConfig.headers" :key="idx" class="form-row">
                    <a-auto-complete
                      v-model="h.key"
                      :data="headerSuggestions"
                      placeholder="Header 名称"
                      style="width: 200px"
                      allow-clear
                    />
                    <a-input v-model="h.value" placeholder="值 (Variable 类型支持 {{变量}})" style="flex: 1" />
                    <a-select v-model="h.type" style="width: 120px">
                      <a-option value="String">String</a-option>
                      <a-option value="Variable">Variable</a-option>
                    </a-select>
                    <a-input v-model="h.description" placeholder="描述 (可选)" style="width: 160px" />
                    <a-button status="danger" @click="httpConfig.headers.splice(idx, 1)">
                      <template #icon><icon-delete /></template>
                    </a-button>
                  </div>
                  <a-button type="dashed" long @click="addHeaderRow">+ 添加 Header</a-button>
                </div>
              </a-form-item>

              <!-- Body -->
              <a-form-item label="Body (请求体字段)">
                <div class="form-list">
                  <div v-for="(b, idx) in httpConfig.body" :key="idx" class="form-row">
                    <a-input v-model="b.key" placeholder="字段名" style="width: 160px" />
                    <a-input v-if="b.type === 'String' || b.type === 'Variable'" v-model="b.value" placeholder="值" style="flex: 1" />
                    <a-input-number v-else-if="b.type === 'Number'" v-model="b.value" placeholder="数值" style="flex: 1" />
                    <a-select v-else-if="b.type === 'Bool'" v-model="b.value" style="flex: 1">
                      <a-option :value="true">true</a-option>
                      <a-option :value="false">false</a-option>
                    </a-select>
                    <a-textarea v-else v-model="b.value" placeholder="JSON 值" :auto-size="{ minRows: 1, maxRows: 3 }" style="flex: 1" />
                    <a-select v-model="b.type" style="width: 120px">
                      <a-option value="String">String</a-option>
                      <a-option value="Number">Number</a-option>
                      <a-option value="Bool">Bool</a-option>
                      <a-option value="Json">Json</a-option>
                      <a-option value="Variable">Variable</a-option>
                    </a-select>
                    <a-input v-model="b.description" placeholder="描述 (可选)" style="width: 160px" />
                    <a-button status="danger" @click="httpConfig.body.splice(idx, 1)">
                      <template #icon><icon-delete /></template>
                    </a-button>
                  </div>
                  <a-button type="dashed" long @click="addBodyRow">+ 添加字段</a-button>
                </div>
              </a-form-item>

              <!-- 用户表单 -->
              <a-divider>用户表单定义</a-divider>
              <a-alert type="info" style="margin-bottom: 16px">
                此处定义的字段将在工作流实例创建时展示给用户填写，值可通过 <code>{<!-- -->{key}}</code> 在 URL / Headers / Body 中引用。
              </a-alert>
              <a-form-item label="Form (用户输入字段)">
                <div class="form-list">
                  <div v-for="(f, idx) in httpConfig.form" :key="idx" class="form-row">
                    <a-input v-model="f.key" placeholder="字段名 (key)" style="width: 160px" />
                    <a-input v-model="f.value" placeholder="默认值 (可选)" style="flex: 1" />
                    <a-select v-model="f.type" style="width: 120px">
                      <a-option value="String">String</a-option>
                      <a-option value="Number">Number</a-option>
                      <a-option value="Bool">Bool</a-option>
                      <a-option value="Json">Json</a-option>
                    </a-select>
                    <a-input v-model="f.description" placeholder="字段说明" style="width: 200px" />
                    <a-button status="danger" @click="httpConfig.form.splice(idx, 1)">
                      <template #icon><icon-delete /></template>
                    </a-button>
                  </div>
                  <a-button type="dashed" long @click="addFormRow">+ 添加表单字段</a-button>
                </div>
              </a-form-item>

              <!-- 运行参数 -->
              <a-divider>运行参数</a-divider>
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

            <!-- ==================== 审批配置 ==================== -->
            <template v-else-if="form.task_type === 'Approval'">
              <a-divider>审批配置</a-divider>
              <a-row :gutter="16">
                <a-col :span="16">
                  <a-form-item label="审批标题" required>
                    <a-input v-model="approvalConfig.title" placeholder="审批标题，支持 {{变量}} 模板" />
                  </a-form-item>
                </a-col>
                <a-col :span="8">
                  <a-form-item label="审批模式" required>
                    <a-select v-model="approvalConfig.approval_mode">
                      <a-option value="Any">抢单模式 (Any)</a-option>
                      <a-option value="All">会签模式 (All)</a-option>
                      <a-option value="Majority">投票模式 (Majority)</a-option>
                    </a-select>
                  </a-form-item>
                </a-col>
              </a-row>
              <a-form-item label="审批说明">
                <a-textarea v-model="approvalConfig.description" :auto-size="{ minRows: 2 }" placeholder="可选" />
              </a-form-item>
              <a-form-item label="审批人规则">
                <div v-for="(rule, idx) in approvalConfig.approvers" :key="idx" style="display: flex; gap: 8px; margin-bottom: 8px">
                  <a-select v-model="rule.type" style="width: 160px">
                    <a-option value="User">指定用户</a-option>
                    <a-option value="Role">指定角色</a-option>
                    <a-option value="ContextVariable">上下文变量</a-option>
                  </a-select>
                  <a-input v-if="rule.type === 'User'" v-model="rule.value" placeholder="user_id" style="flex: 1" />
                  <a-select v-else-if="rule.type === 'Role'" v-model="rule.value" style="flex: 1">
                    <a-option value="TenantAdmin">TenantAdmin</a-option>
                    <a-option value="Developer">Developer</a-option>
                    <a-option value="Operator">Operator</a-option>
                  </a-select>
                  <a-input v-else v-model="rule.value" placeholder="变量名" style="flex: 1" />
                  <a-button status="danger" @click="approvalConfig.approvers.splice(idx, 1)">删除</a-button>
                </div>
                <a-button type="dashed" long @click="approvalConfig.approvers.push({ type: 'User', value: '' })">
                  + 添加审批人规则
                </a-button>
              </a-form-item>
              <a-form-item label="超时 (秒)">
                <a-input-number v-model="approvalConfig.timeout" :min="0" placeholder="不填则不超时" />
              </a-form-item>
            </template>

            <!-- ==================== gRPC 预留 ==================== -->
            <template v-else-if="form.task_type === 'Grpc'">
              <a-divider>gRPC 配置</a-divider>
              <a-alert type="info">gRPC 类型配置即将支持</a-alert>
            </template>

            <a-form-item>
              <a-space>
                <a-button type="primary" html-type="submit" :loading="saving">保存</a-button>
                <a-button @click="$router.push('/tasks')">取消</a-button>
              </a-space>
            </a-form-item>
          </a-form>
        </a-card>
      </a-tab-pane>

      <a-tab-pane v-if="isEdit" key="instances" title="实例列表">
        <a-card>
          <a-form :model="instanceFilters" layout="inline" class="filter-form">
            <a-form-item label="状态">
              <a-select v-model="instanceFilters.status" allow-clear placeholder="全部" style="width: 140px">
                <a-option value="Pending">Pending</a-option>
                <a-option value="Running">Running</a-option>
                <a-option value="Completed">Completed</a-option>
                <a-option value="Failed">Failed</a-option>
                <a-option value="Canceled">Canceled</a-option>
              </a-select>
            </a-form-item>
            <a-form-item>
              <a-space>
                <a-button type="primary" @click="onSearchInstances">查询</a-button>
                <a-button @click="onResetInstanceFilters">重置</a-button>
              </a-space>
            </a-form-item>
          </a-form>

          <a-table
            :data="taskInstances"
            :columns="instanceColumns"
            :loading="instancesLoading"
            row-key="id"
            :pagination="instancePagination"
            @page-change="onInstancePageChange"
            @page-size-change="onInstancePageSizeChange"
          >
            <template #task_status="{ record }">
              <status-tag :status="record.task_status" :map="TASK_INSTANCE_STATUS_MAP" />
            </template>
            <template #duration="{ record }">{{ formatDuration(record.execution_duration) }}</template>
            <template #created_at="{ record }">{{ formatDate(record.created_at) }}</template>
            <template #action="{ record }">
              <a-button type="text" size="small" @click="$router.push(`/tasks/instances/${record.id}`)">详情</a-button>
            </template>
          </a-table>
        </a-card>
      </a-tab-pane>
    </a-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { taskApi } from '../../../api/task'
import { taskInstanceApi } from '../../../api/task-instance'
import { TASK_INSTANCE_STATUS_MAP } from '../../../utils/constants'
import { formatDate, formatDuration } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import { IconDelete } from '@arco-design/web-vue/es/icon'
import type { TaskEntity, TaskInstanceEntity, TaskInstanceStatus, HttpMethod, FormField, FormValueType, ListTaskInstancesParams } from '../../../types/task'

const route = useRoute()
const router = useRouter()
const isEdit = computed(() => !!route.params.id)
const taskId = computed(() => route.params.id as string)
const pageLoading = ref(false)
const saving = ref(false)

const form = reactive({
  name: '',
  task_type: 'Http' as string,
  description: '',
  status: 'Draft' as string,
})

interface FormRow {
  key: string
  value: any
  type: FormValueType
  description: string
}

const httpConfig = reactive({
  url: '',
  method: 'Get' as HttpMethod,
  headers: [] as FormRow[],
  body: [] as FormRow[],
  form: [] as FormRow[],
  retry_count: 0,
  retry_delay: 0,
  timeout: 30,
  success_condition: null as string | null,
})

const approvalConfig = reactive({
  title: '',
  description: '' as string,
  approval_mode: 'Any' as string,
  approvers: [] as { type: string; value: string }[],
  timeout: null as number | null,
})

// ---- Header 预设与建议 ----

const headerPresets = [
  { key: 'Content-Type', value: 'application/json', type: 'String' as FormValueType },
  { key: 'Authorization', value: 'Bearer ', type: 'Variable' as FormValueType },
  { key: 'Accept', value: 'application/json', type: 'String' as FormValueType },
  { key: 'X-API-Key', value: '', type: 'Variable' as FormValueType },
]

const headerSuggestions = [
  'Content-Type', 'Authorization', 'Accept', 'X-API-Key',
  'Cache-Control', 'User-Agent', 'X-Request-ID', 'X-Forwarded-For',
  'If-None-Match', 'Origin', 'Referer',
]

function addPresetHeader(preset: { key: string; value: string; type: FormValueType }) {
  const exists = httpConfig.headers.some(h => h.key === preset.key)
  if (exists) {
    Notification.warning({ content: `Header "${preset.key}" 已存在` })
    return
  }
  httpConfig.headers.push({ key: preset.key, value: preset.value, type: preset.type, description: '' })
}

function addHeaderRow() {
  httpConfig.headers.push({ key: '', value: '', type: 'String', description: '' })
}

function addBodyRow() {
  httpConfig.body.push({ key: '', value: '', type: 'String', description: '' })
}

function addFormRow() {
  httpConfig.form.push({ key: '', value: '', type: 'String', description: '' })
}

// ---- 构建提交数据 ----

function toFormFields(rows: FormRow[]): FormField[] {
  return rows
    .filter(r => r.key.trim() !== '')
    .map(r => {
      const field: FormField = { key: r.key, value: r.value, type: r.type }
      if (r.description) field.description = r.description
      return field
    })
}

function buildTaskTemplate() {
  if (form.task_type === 'Http') {
    return {
      Http: {
        url: httpConfig.url,
        method: httpConfig.method,
        headers: toFormFields(httpConfig.headers),
        body: toFormFields(httpConfig.body),
        form: toFormFields(httpConfig.form),
        retry_count: httpConfig.retry_count,
        retry_delay: httpConfig.retry_delay,
        timeout: httpConfig.timeout,
        success_condition: httpConfig.success_condition || null,
      },
    }
  }
  if (form.task_type === 'Approval') {
    return {
      Approval: {
        name: form.name,
        title: approvalConfig.title,
        description: approvalConfig.description || null,
        approvers: approvalConfig.approvers.map(r => {
          if (r.type === 'User') return { User: r.value }
          if (r.type === 'Role') return { Role: r.value }
          return { ContextVariable: r.value }
        }),
        approval_mode: approvalConfig.approval_mode,
        timeout: approvalConfig.timeout || null,
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

// ---- 编辑模式加载 ----

function formFieldsToRows(fields: FormField[] | undefined | null): FormRow[] {
  if (!fields || !Array.isArray(fields)) return []
  return fields.map(f => ({
    key: f.key,
    value: f.value,
    type: f.type || 'String',
    description: f.description || '',
  }))
}

// ---- 实例列表 ----

const taskInstances = ref<TaskInstanceEntity[]>([])
const instancesLoading = ref(false)
const instancesLoaded = ref(false)

const instanceFilters = reactive<{
  status: TaskInstanceStatus | undefined
}>({
  status: undefined,
})

const instancePagination = reactive({
  current: 1,
  pageSize: 10,
  total: 0,
  showTotal: true,
  showPageSize: true,
  pageSizeOptions: [10, 20, 50, 100],
})

const instanceColumns = [
  { title: '实例ID', dataIndex: 'task_instance_id', ellipsis: true, width: 200 },
  { title: '状态', slotName: 'task_status', width: 100 },
  { title: '耗时', slotName: 'duration', width: 100 },
  { title: '创建时间', slotName: 'created_at', width: 180 },
  { title: '操作', slotName: 'action', width: 100 },
]

function buildInstanceParams(): ListTaskInstancesParams {
  const p: ListTaskInstancesParams = {
    task_id: taskId.value,
    page: instancePagination.current,
    page_size: instancePagination.pageSize,
  }
  if (instanceFilters.status) p.status = instanceFilters.status
  return p
}

async function fetchInstances() {
  instancesLoading.value = true
  try {
    const res = await taskInstanceApi.list(buildInstanceParams())
    taskInstances.value = res.data.items
    instancePagination.total = Number(res.data.total)
  } catch {} finally { instancesLoading.value = false }
}

function onInstancePageChange(page: number) {
  instancePagination.current = page
  fetchInstances()
}

function onInstancePageSizeChange(size: number) {
  instancePagination.pageSize = size
  instancePagination.current = 1
  fetchInstances()
}

function onSearchInstances() {
  instancePagination.current = 1
  fetchInstances()
}

function onResetInstanceFilters() {
  instanceFilters.status = undefined
  instancePagination.current = 1
  fetchInstances()
}

function handleTabChange(key: string) {
  if (key === 'instances' && !instancesLoaded.value) {
    instancesLoaded.value = true
    fetchInstances()
  }
}

// ---- 初始化 ----

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
        httpConfig.url = tpl.url
        httpConfig.method = tpl.method
        httpConfig.headers = formFieldsToRows(tpl.headers)
        httpConfig.body = formFieldsToRows(tpl.body)
        httpConfig.form = formFieldsToRows(tpl.form)
        httpConfig.retry_count = tpl.retry_count
        httpConfig.retry_delay = tpl.retry_delay
        httpConfig.timeout = tpl.timeout
        httpConfig.success_condition = tpl.success_condition
      } else if (entity.task_type === 'Approval' && typeof entity.task_template === 'object' && 'Approval' in entity.task_template) {
        const tpl = entity.task_template.Approval
        approvalConfig.title = tpl.title
        approvalConfig.description = tpl.description || ''
        approvalConfig.approval_mode = tpl.approval_mode
        approvalConfig.timeout = tpl.timeout
        approvalConfig.approvers = tpl.approvers.map((r: any) => {
          if (r.User !== undefined) return { type: 'User', value: r.User }
          if (r.Role !== undefined) return { type: 'Role', value: r.Role }
          if (r.ContextVariable !== undefined) return { type: 'ContextVariable', value: r.ContextVariable }
          return { type: 'User', value: '' }
        })
      }
    } catch {} finally { pageLoading.value = false }
  }
})
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

.preset-tags {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.preset-label {
  font-size: 13px;
  color: var(--color-text-3);
}

.preset-tag {
  cursor: pointer;
}

.filter-form {
  margin-bottom: 16px;
}
</style>
