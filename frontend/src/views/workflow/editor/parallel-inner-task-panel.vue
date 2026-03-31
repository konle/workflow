<template>
  <div>
    <a-divider>子任务</a-divider>
    <a-form-item label="子任务类型">
      <a-select
        :model-value="kind"
        :disabled="readonly"
        @update:model-value="onKindChange"
      >
        <a-option value="Http">HTTP</a-option>
        <a-option value="Grpc">gRPC</a-option>
        <a-option value="SubWorkflow">子工作流</a-option>
      </a-select>
    </a-form-item>

    <PublishedTaskRefFields
      v-if="kind === 'Http' || kind === 'Grpc'"
      section-title="任务选择"
      :task-type="kind"
      v-model:task-id="innerTaskId"
      v-model:form-fields="innerFormFields"
      :tasks="tasksForInnerKind"
      :task-snapshot="innerSnapshot"
      :readonly="readonly"
      @change="onInnerPublishedTaskChange"
      @update:form-fields="onInnerFormFieldsUpdate"
    />

    <SubworkflowRefFields
      v-else
      section-title="工作流选择"
      v-model:meta-id="subMetaId"
      v-model:version="subVersion"
      v-model:timeout="subTimeout"
      v-model:form-fields="subFormFields"
      :sub-workflow-meta="subMeta"
      :sub-workflow-versions="subVersions"
      :workflow-metas="workflowMetas"
      :readonly="readonly"
      @meta-change="onParallelSubMetaChange"
      @version-change="emit('change')"
      @update:timeout="emit('change')"
      @update:form-fields="emit('change')"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { TaskEntity } from '../../../types/task'
import type { WorkflowMetaEntity } from '../../../types/workflow'
import { workflowApi } from '../../../api/workflow'
import PublishedTaskRefFields from './published-task-ref-fields.vue'
import SubworkflowRefFields from './subworkflow-ref-fields.vue'
import { buildFormFields } from './workflow-editor-form-utils'
import {
  defaultParallelInnerTemplate,
  type ParallelInnerKind,
} from './parallel-inner-task-utils'

const props = defineProps<{
  nodeData: Record<string, unknown>
  taskCache: TaskEntity[]
  workflowMetas: WorkflowMetaEntity[]
  readonly?: boolean
}>()

const emit = defineEmits<{ change: [] }>()

const readonly = computed(() => props.readonly ?? false)

const kind = computed(() => (props.nodeData.parallelInnerKind || 'Http') as ParallelInnerKind)

const innerTaskId = computed({
  get: () => (props.nodeData.parallelInnerTaskId as string | null) ?? null,
  set: (v: string | null) => {
    props.nodeData.parallelInnerTaskId = v
  },
})

const innerFormFields = computed({
  get: () => (props.nodeData.parallelInnerFormFields as ReturnType<typeof buildFormFields>) || [],
  set: (v) => {
    props.nodeData.parallelInnerFormFields = v
  },
})

const innerSnapshot = computed(() => props.nodeData.parallelInnerSnapshot as Record<string, unknown> | string | null)

const subMetaId = computed({
  get: () => (props.nodeData.parallelSubWorkflowMetaId as string | null) ?? null,
  set: (v: string | null) => {
    props.nodeData.parallelSubWorkflowMetaId = v
  },
})

const subVersion = computed({
  get: () => (props.nodeData.parallelSubWorkflowVersion as number | null) ?? null,
  set: (v: number | null) => {
    props.nodeData.parallelSubWorkflowVersion = v
  },
})

const subTimeout = computed({
  get: () => (props.nodeData.parallelSubWorkflowTimeout as number | null | undefined) ?? null,
  set: (v: number | null | undefined) => {
    props.nodeData.parallelSubWorkflowTimeout = v ?? null
  },
})

const subFormFields = computed({
  get: () => (props.nodeData.parallelSubWorkflowFormFields as ReturnType<typeof buildFormFields>) || [],
  set: (v) => {
    props.nodeData.parallelSubWorkflowFormFields = v
  },
})

const subMeta = computed(() => props.nodeData.parallelSubWorkflowMeta as WorkflowMetaEntity | null)
const subVersions = computed(
  () =>
    (props.nodeData.parallelSubWorkflowVersions as { version: number; nodes?: unknown[] }[]) || [],
)

const tasksForInnerKind = computed(() =>
  props.taskCache.filter(t => t.task_type === kind.value && t.status === 'Published'),
)

function syncInnerTemplateFromTask() {
  const d = props.nodeData
  const k = kind.value
  const id = d.parallelInnerTaskId as string | null
  if (!id) {
    d.parallelInnerSnapshot = null
    d.parallelInnerFormFields = []
    return
  }
  const task = props.taskCache.find(t => t.id === id)
  if (!task) {
    d.parallelInnerSnapshot = null
    d.parallelInnerFormFields = []
    return
  }
  d.parallelInnerSnapshot = task.task_template as Record<string, unknown>
  const cfg = d.config as { task_template?: unknown }
  cfg.task_template = JSON.parse(JSON.stringify(task.task_template)) as typeof cfg.task_template
  const tpl = task.task_template as Record<string, unknown> | string
  if (typeof tpl === 'string') {
    d.parallelInnerFormFields = []
    return
  }
  if (tpl !== null && k in tpl) {
    const inner = (tpl as Record<string, { form?: unknown[] }>)[k]
    d.parallelInnerFormFields = inner?.form?.length ? buildFormFields(inner.form as Parameters<typeof buildFormFields>[0]) : []
  } else {
    d.parallelInnerFormFields = []
  }
}

function onInnerPublishedTaskChange() {
  syncInnerTemplateFromTask()
  emit('change')
}

function onInnerFormFieldsUpdate() {
  emit('change')
}

function onKindChange(newKind: ParallelInnerKind) {
  const d = props.nodeData
  d.parallelInnerKind = newKind
  d.config = d.config || {}
  const cfg = d.config as { task_template?: unknown }
  cfg.task_template = defaultParallelInnerTemplate(newKind)
  d.parallelInnerTaskId = null
  d.parallelInnerSnapshot = null
  d.parallelInnerFormFields = []
  d.parallelSubWorkflowMetaId = null
  d.parallelSubWorkflowVersion = null
  d.parallelSubWorkflowMeta = null
  d.parallelSubWorkflowVersions = []
  d.parallelSubWorkflowFormFields = []
  d.parallelSubWorkflowTimeout = null
  emit('change')
}

async function onParallelSubMetaChange() {
  const d = props.nodeData
  const id = d.parallelSubWorkflowMetaId as string | null
  if (!id) {
    d.parallelSubWorkflowMeta = null
    d.parallelSubWorkflowVersions = []
    d.parallelSubWorkflowVersion = null
    d.parallelSubWorkflowFormFields = []
    emit('change')
    return
  }
  const meta = props.workflowMetas.find(m => m.workflow_meta_id === id)
  d.parallelSubWorkflowMeta = meta || null
  try {
    const res = await workflowApi.listTemplates(id)
    d.parallelSubWorkflowVersions = res.data
    if (res.data.length > 0) d.parallelSubWorkflowVersion = res.data[res.data.length - 1].version
  } catch {
    d.parallelSubWorkflowVersions = []
  }
  d.parallelSubWorkflowFormFields = meta?.form?.length ? buildFormFields(meta.form) : []
  const cfg = d.config as { task_template?: unknown }
  cfg.task_template = {
    SubWorkflow: {
      workflow_meta_id: id,
      workflow_version: (d.parallelSubWorkflowVersion as number) || 1,
      form: [],
      timeout: (d.parallelSubWorkflowTimeout as number | null) ?? null,
    },
  }
  emit('change')
}
</script>
