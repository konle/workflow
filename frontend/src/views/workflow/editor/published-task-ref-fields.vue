<template>
  <div>
    <a-divider>{{ sectionTitle }}</a-divider>
    <a-form-item label="选择任务模板">
      <a-select
        :model-value="taskId"
        placeholder="搜索并选择已有任务"
        allow-search
        :disabled="readonly"
        @update:model-value="onTaskIdUpdate"
      >
        <a-option v-for="t in tasks" :key="t.id" :value="t.id">{{ t.name }} ({{ t.status }})</a-option>
      </a-select>
    </a-form-item>

    <template v-if="taskSnapshot">
      <a-divider>任务信息</a-divider>
      <div class="task-info">
        <template v-if="taskType === 'Http' && taskSnapshot.Http">
          <a-descriptions :column="1" size="small" bordered>
            <a-descriptions-item label="URL">{{ taskSnapshot.Http.url }}</a-descriptions-item>
            <a-descriptions-item label="Method">{{ taskSnapshot.Http.method }}</a-descriptions-item>
            <a-descriptions-item label="超时">{{ taskSnapshot.Http.timeout }}s</a-descriptions-item>
            <a-descriptions-item label="重试">{{ taskSnapshot.Http.retry_count }}次</a-descriptions-item>
          </a-descriptions>
        </template>
        <template v-else-if="taskType === 'Approval' && taskSnapshot.Approval">
          <a-descriptions :column="1" size="small" bordered>
            <a-descriptions-item label="标题">{{ taskSnapshot.Approval.title }}</a-descriptions-item>
            <a-descriptions-item label="模式">{{ taskSnapshot.Approval.approval_mode }}</a-descriptions-item>
          </a-descriptions>
        </template>
        <template v-else-if="taskType === 'Llm' && taskSnapshot.Llm">
          <a-descriptions :column="1" size="small" bordered>
            <a-descriptions-item label="Base URL">{{ taskSnapshot.Llm.base_url }}</a-descriptions-item>
            <a-descriptions-item label="模型">{{ taskSnapshot.Llm.model }}</a-descriptions-item>
            <a-descriptions-item label="API Key 引用">{{ taskSnapshot.Llm.api_key_ref }}</a-descriptions-item>
            <a-descriptions-item label="超时">{{ taskSnapshot.Llm.timeout }}s</a-descriptions-item>
            <a-descriptions-item label="重试">{{ taskSnapshot.Llm.retry_count }}次</a-descriptions-item>
          </a-descriptions>
        </template>
        <template v-else-if="taskType === 'Grpc'">
          <a-descriptions :column="1" size="small" bordered>
            <a-descriptions-item label="类型">gRPC</a-descriptions-item>
          </a-descriptions>
        </template>
      </div>

      <template v-if="formFields?.length">
        <a-divider>运行参数</a-divider>
        <div class="form-list">
          <div v-for="(f, idx) in formFields" :key="idx" class="form-row">
            <a-input :model-value="f.key" disabled style="width: 120px" />
            <a-input
              :model-value="f.value"
              :placeholder="f.description || '值'"
              style="flex: 1"
              :disabled="readonly"
              @update:model-value="onFieldValue(idx, $event)"
            />
            <a-select
              :model-value="f.type"
              style="width: 110px"
              :disabled="readonly"
              @update:model-value="onFieldType(idx, $event)"
            >
              <a-option :value="f.originalType">{{ f.originalType }}</a-option>
              <a-option v-if="f.originalType !== 'Variable'" value="Variable">Variable</a-option>
            </a-select>
          </div>
        </div>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import type { TaskEntity } from '../../../types/task'
import type { EditorFormField } from './workflow-editor-form-utils'

const props = withDefaults(
  defineProps<{
    sectionTitle?: string
    taskType: 'Http' | 'Approval' | 'Grpc' | 'Llm'
    taskId: string | null
    tasks: TaskEntity[]
    /** Task template payload; may be a string (e.g. gRPC). */
    taskSnapshot: any
    formFields: EditorFormField[]
    readonly?: boolean
  }>(),
  { sectionTitle: '任务选择', readonly: false },
)

const emit = defineEmits<{
  'update:taskId': [id: string | null]
  'update:formFields': [fields: EditorFormField[]]
  change: []
}>()

function onTaskIdUpdate(id: string | null) {
  emit('update:taskId', id)
  emit('change')
}

function onFieldValue(index: number, value: unknown) {
  const next = props.formFields.map((f, i) => (i === index ? { ...f, value } : f))
  emit('update:formFields', next)
}

function onFieldType(index: number, type: string) {
  const next = props.formFields.map((f, i) => (i === index ? { ...f, type } : f))
  emit('update:formFields', next)
}
</script>

<style scoped>
.task-info {
  margin-bottom: 8px;
}
.form-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}
.form-row {
  display: flex;
  gap: 6px;
  align-items: flex-start;
}
</style>
