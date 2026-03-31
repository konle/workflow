<template>
  <div>
    <a-divider>{{ sectionTitle }}</a-divider>
    <a-form-item label="选择工作流">
      <a-select
        :model-value="metaId"
        placeholder="搜索并选择已有工作流"
        allow-search
        :disabled="readonly"
        @update:model-value="onMetaUpdate"
      >
        <a-option v-for="m in workflowMetas" :key="m.workflow_meta_id" :value="m.workflow_meta_id">
          {{ m.name }} ({{ m.status }})
        </a-option>
      </a-select>
    </a-form-item>
    <a-form-item v-if="subWorkflowVersions?.length" label="版本">
      <a-select
        :model-value="version"
        :disabled="readonly"
        @update:model-value="onVersionUpdate"
      >
        <a-option v-for="v in subWorkflowVersions" :key="v.version" :value="v.version">
          v{{ v.version }} ({{ v.nodes?.length || 0 }}节点)
        </a-option>
      </a-select>
    </a-form-item>
    <template v-if="subWorkflowMeta">
      <a-divider>工作流信息</a-divider>
      <a-descriptions :column="1" size="small" bordered>
        <a-descriptions-item label="名称">{{ subWorkflowMeta.name }}</a-descriptions-item>
        <a-descriptions-item label="状态">{{ subWorkflowMeta.status }}</a-descriptions-item>
      </a-descriptions>
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
              @update:model-value="(v: unknown) => onFieldValue(idx, v)"
            />
            <a-select
              :model-value="f.type"
              style="width: 110px"
              :disabled="readonly"
              @update:model-value="(t: string) => onFieldType(idx, t)"
            >
              <a-option :value="f.originalType">{{ f.originalType }}</a-option>
              <a-option v-if="f.originalType !== 'Variable'" value="Variable">Variable</a-option>
            </a-select>
          </div>
        </div>
      </template>
    </template>
    <a-form-item label="超时(秒)">
      <a-input-number
        :model-value="timeout ?? undefined"
        :min="0"
        placeholder="不填则不超时"
        :disabled="readonly"
        @update:model-value="onTimeoutUpdate"
      />
    </a-form-item>
  </div>
</template>

<script setup lang="ts">
import type { WorkflowMetaEntity } from '../../../types/workflow'
import type { EditorFormField } from './workflow-editor-form-utils'

const props = withDefaults(
  defineProps<{
    sectionTitle?: string
    metaId: string | null
    version: number | null
    subWorkflowMeta: WorkflowMetaEntity | null
    subWorkflowVersions: { version: number; nodes?: unknown[] }[]
    workflowMetas: WorkflowMetaEntity[]
    formFields: EditorFormField[]
    timeout: number | null | undefined
    readonly?: boolean
  }>(),
  { sectionTitle: '工作流选择', readonly: false },
)

const emit = defineEmits<{
  'update:metaId': [id: string | null]
  'update:version': [v: number | null]
  'update:timeout': [t: number | null | undefined]
  'update:formFields': [fields: EditorFormField[]]
  'meta-change': []
  'version-change': []
}>()

function onMetaUpdate(id: string | null) {
  emit('update:metaId', id)
  emit('meta-change')
}

function onVersionUpdate(v: number | null) {
  emit('update:version', v)
  emit('version-change')
}

function onTimeoutUpdate(v: number | undefined) {
  emit('update:timeout', v === undefined ? null : v)
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
