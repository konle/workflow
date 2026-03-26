<template>
  <div class="editor-container">
    <div class="editor-header">
      <a-button @click="$router.back()">← 返回</a-button>
      <span class="editor-title">{{ metaName }} <a-tag v-if="currentVersion">v{{ currentVersion }}</a-tag></span>
      <a-space>
        <a-button type="primary" @click="handleSave" :loading="saving">保存</a-button>
      </a-space>
    </div>

    <div class="editor-body">
      <div class="editor-panel-left">
        <div class="panel-title">节点面板</div>
        <div
          v-for="nt in nodeTypes"
          :key="nt.type"
          class="node-item"
          :style="{ borderLeftColor: nt.color }"
          draggable="true"
          @dragstart="onDragStart($event, nt)"
        >
          <span class="node-dot" :style="{ background: nt.color }"></span>
          {{ nt.label }}
        </div>
      </div>

      <div class="editor-canvas" @drop="onDrop" @dragover.prevent>
        <VueFlow
          v-model:nodes="nodes"
          v-model:edges="edges"
          @node-click="onNodeClick"
          fit-view-on-init
          :default-edge-options="{ type: 'smoothstep' }"
        >
          <Background />
          <Controls />
          <MiniMap />
        </VueFlow>
      </div>

      <div class="editor-panel-right">
        <div class="panel-title">属性面板</div>
        <template v-if="selectedNode">
          <a-form :model="selectedNode.data" layout="vertical" size="small">
            <a-form-item label="节点ID">
              <a-input :model-value="selectedNode.id" disabled />
            </a-form-item>
            <a-form-item label="节点类型">
              <a-input :model-value="selectedNode.data.nodeType" disabled />
            </a-form-item>

            <!-- ===== 引用原子任务: Http / Approval / gRPC ===== -->
            <template v-if="isTaskRefNode(selectedNode.data.nodeType)">
              <a-divider>任务选择</a-divider>
              <a-form-item label="选择任务模板">
                <a-select
                  v-model="selectedNode.data.taskId"
                  placeholder="搜索并选择已有任务"
                  allow-search
                  @change="onTaskSelected(selectedNode)"
                >
                  <a-option
                    v-for="t in getTasksForType(selectedNode.data.nodeType)"
                    :key="t.id"
                    :value="t.id"
                  >{{ t.name }} ({{ t.status }})</a-option>
                </a-select>
              </a-form-item>

              <template v-if="selectedNode.data.taskSnapshot">
                <a-divider>任务信息</a-divider>
                <div class="task-info">
                  <template v-if="selectedNode.data.nodeType === 'Http' && selectedNode.data.taskSnapshot.Http">
                    <a-descriptions :column="1" size="small" bordered>
                      <a-descriptions-item label="URL">{{ selectedNode.data.taskSnapshot.Http.url }}</a-descriptions-item>
                      <a-descriptions-item label="Method">{{ selectedNode.data.taskSnapshot.Http.method }}</a-descriptions-item>
                      <a-descriptions-item label="超时">{{ selectedNode.data.taskSnapshot.Http.timeout }}s</a-descriptions-item>
                      <a-descriptions-item label="重试">{{ selectedNode.data.taskSnapshot.Http.retry_count }}次</a-descriptions-item>
                    </a-descriptions>
                  </template>
                  <template v-else-if="selectedNode.data.nodeType === 'Approval' && selectedNode.data.taskSnapshot.Approval">
                    <a-descriptions :column="1" size="small" bordered>
                      <a-descriptions-item label="标题">{{ selectedNode.data.taskSnapshot.Approval.title }}</a-descriptions-item>
                      <a-descriptions-item label="模式">{{ selectedNode.data.taskSnapshot.Approval.approval_mode }}</a-descriptions-item>
                    </a-descriptions>
                  </template>
                  <template v-else-if="selectedNode.data.nodeType === 'Grpc'">
                    <a-descriptions :column="1" size="small" bordered>
                      <a-descriptions-item label="类型">gRPC</a-descriptions-item>
                    </a-descriptions>
                  </template>
                </div>

                <template v-if="selectedNode.data.formFields?.length">
                  <a-divider>运行参数</a-divider>
                  <div class="form-list">
                    <div v-for="(f, idx) in selectedNode.data.formFields" :key="idx" class="form-row">
                      <a-input :model-value="f.key" disabled style="width: 120px" />
                      <a-input v-model="f.value" :placeholder="f.description || '值'" style="flex: 1" />
                      <a-select v-model="f.type" style="width: 110px">
                        <a-option :value="f.originalType">{{ f.originalType }}</a-option>
                        <a-option v-if="f.originalType !== 'Variable'" value="Variable">Variable</a-option>
                      </a-select>
                    </div>
                  </div>
                </template>
              </template>
            </template>

            <!-- ===== 引用工作流: SubWorkflow ===== -->
            <template v-else-if="selectedNode.data.nodeType === 'SubWorkflow'">
              <a-divider>工作流选择</a-divider>
              <a-form-item label="选择工作流">
                <a-select
                  v-model="selectedNode.data.subWorkflowMetaId"
                  placeholder="搜索并选择已有工作流"
                  allow-search
                  @change="onSubWorkflowMetaSelected(selectedNode)"
                >
                  <a-option
                    v-for="m in workflowMetas"
                    :key="m.workflow_meta_id"
                    :value="m.workflow_meta_id"
                  >{{ m.name }} ({{ m.status }})</a-option>
                </a-select>
              </a-form-item>
              <a-form-item v-if="selectedNode.data.subWorkflowVersions?.length" label="版本">
                <a-select
                  v-model="selectedNode.data.subWorkflowVersion"
                  @change="onSubWorkflowVersionSelected(selectedNode)"
                >
                  <a-option
                    v-for="v in selectedNode.data.subWorkflowVersions"
                    :key="v.version"
                    :value="v.version"
                  >v{{ v.version }} ({{ v.nodes?.length || 0 }}节点)</a-option>
                </a-select>
              </a-form-item>

              <template v-if="selectedNode.data.subWorkflowMeta">
                <a-divider>工作流信息</a-divider>
                <a-descriptions :column="1" size="small" bordered>
                  <a-descriptions-item label="名称">{{ selectedNode.data.subWorkflowMeta.name }}</a-descriptions-item>
                  <a-descriptions-item label="状态">{{ selectedNode.data.subWorkflowMeta.status }}</a-descriptions-item>
                </a-descriptions>

                <template v-if="selectedNode.data.formFields?.length">
                  <a-divider>运行参数</a-divider>
                  <div class="form-list">
                    <div v-for="(f, idx) in selectedNode.data.formFields" :key="idx" class="form-row">
                      <a-input :model-value="f.key" disabled style="width: 120px" />
                      <a-input v-model="f.value" :placeholder="f.description || '值'" style="flex: 1" />
                      <a-select v-model="f.type" style="width: 110px">
                        <a-option :value="f.originalType">{{ f.originalType }}</a-option>
                        <a-option v-if="f.originalType !== 'Variable'" value="Variable">Variable</a-option>
                      </a-select>
                    </div>
                  </div>
                </template>
              </template>
              <a-form-item label="超时(秒)">
                <a-input-number v-model="selectedNode.data.config.timeout" :min="0" placeholder="不填则不超时" />
              </a-form-item>
            </template>

            <!-- ===== 控制流: IfCondition ===== -->
            <template v-else-if="selectedNode.data.nodeType === 'IfCondition'">
              <a-form-item label="条件名称">
                <a-input v-model="selectedNode.data.config.name" />
              </a-form-item>
              <a-form-item label="条件表达式 (Rhai)">
                <a-textarea v-model="selectedNode.data.config.condition" :auto-size="{ minRows: 3 }" />
              </a-form-item>
              <a-form-item label="Then 节点ID">
                <a-input v-model="selectedNode.data.config.then_task" />
              </a-form-item>
              <a-form-item label="Else 节点ID">
                <a-input v-model="selectedNode.data.config.else_task" />
              </a-form-item>
            </template>

            <!-- ===== 控制流: ContextRewrite ===== -->
            <template v-else-if="selectedNode.data.nodeType === 'ContextRewrite'">
              <a-form-item label="名称">
                <a-input v-model="selectedNode.data.config.name" />
              </a-form-item>
              <a-form-item label="Rhai 脚本">
                <a-textarea v-model="selectedNode.data.config.script" :auto-size="{ minRows: 4 }" />
              </a-form-item>
              <a-form-item label="合并模式">
                <a-select v-model="selectedNode.data.config.merge_mode">
                  <a-option value="Merge">Merge</a-option>
                  <a-option value="Replace">Replace</a-option>
                </a-select>
              </a-form-item>
            </template>

            <!-- ===== 控制流: Parallel ===== -->
            <template v-else-if="selectedNode.data.nodeType === 'Parallel'">
              <a-form-item label="数据路径 (items_path)">
                <a-input v-model="selectedNode.data.config.items_path" />
              </a-form-item>
              <a-form-item label="迭代变量名 (item_alias)">
                <a-input v-model="selectedNode.data.config.item_alias" />
              </a-form-item>
              <a-form-item label="并发度">
                <a-input-number v-model="selectedNode.data.config.concurrency" :min="1" />
              </a-form-item>
              <a-form-item label="模式">
                <a-select v-model="selectedNode.data.config.mode">
                  <a-option value="Rolling">Rolling</a-option>
                  <a-option value="Batch">Batch</a-option>
                </a-select>
              </a-form-item>
              <a-form-item label="最大失败数">
                <a-input-number v-model="selectedNode.data.config.max_failures" :min="0" />
              </a-form-item>
            </template>

            <!-- ===== 控制流: ForkJoin ===== -->
            <template v-else-if="selectedNode.data.nodeType === 'ForkJoin'">
              <a-form-item label="并发度">
                <a-input-number v-model="selectedNode.data.config.concurrency" :min="1" />
              </a-form-item>
              <a-form-item label="模式">
                <a-select v-model="selectedNode.data.config.mode">
                  <a-option value="Rolling">Rolling</a-option>
                  <a-option value="Batch">Batch</a-option>
                </a-select>
              </a-form-item>
              <a-form-item label="子任务 (JSON)">
                <a-textarea v-model="selectedNode.data.config.tasksJson" :auto-size="{ minRows: 4 }" />
              </a-form-item>
            </template>

            <a-form-item label="Context (JSON)">
              <a-textarea v-model="selectedNode.data.contextJson" :auto-size="{ minRows: 2 }" />
            </a-form-item>
          </a-form>
        </template>
        <a-empty v-else description="点击节点编辑属性" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { VueFlow, useVueFlow, type Node, type Edge } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import { workflowApi } from '../../../api/workflow'
import { taskApi } from '../../../api/task'
import { Notification } from '@arco-design/web-vue'
import type { TaskEntity, FormField } from '../../../types/task'
import type { WorkflowMetaEntity, WorkflowEntity } from '../../../types/workflow'
import dagre from 'dagre'

const route = useRoute()
const router = useRouter()
const metaId = route.params.metaId as string
const versionParam = route.params.version ? Number(route.params.version) : null

const metaName = ref('')
const currentVersion = ref(versionParam || 0)
const saving = ref(false)

const nodes = ref<Node[]>([])
const edges = ref<Edge[]>([])
const selectedNode = ref<Node | null>(null)

const taskCache = ref<TaskEntity[]>([])
const workflowMetas = ref<WorkflowMetaEntity[]>([])

let nodeCounter = 0

const TASK_REF_TYPES = ['Http', 'Approval', 'Grpc']
function isTaskRefNode(nodeType: string) {
  return TASK_REF_TYPES.includes(nodeType)
}

function getTasksForType(nodeType: string): TaskEntity[] {
  return taskCache.value.filter(t => t.task_type === nodeType && t.status === 'Published')
}

const nodeTypes = [
  { type: 'Http', label: 'HTTP', color: '#3491FA' },
  { type: 'IfCondition', label: '条件分支', color: '#F7BA1E' },
  { type: 'ContextRewrite', label: '上下文重写', color: '#14C9C9' },
  { type: 'Parallel', label: '并发容器', color: '#00B42A' },
  { type: 'ForkJoin', label: '异构并发', color: '#009A29' },
  { type: 'SubWorkflow', label: '子工作流', color: '#86909C' },
  { type: 'Grpc', label: 'gRPC', color: '#722ED1' },
  { type: 'Approval', label: '审批', color: '#F77234' },
]

interface EditorFormField {
  key: string
  value: any
  type: string
  originalType: string
  description: string
}

function getDefaultConfig(type: string): any {
  switch (type) {
    case 'Http': return {}
    case 'Approval': return {}
    case 'Grpc': return {}
    case 'SubWorkflow': return { timeout: null }
    case 'IfCondition': return { name: '', condition: '', then_task: '', else_task: '' }
    case 'ContextRewrite': return { name: '', script: '', merge_mode: 'Merge' }
    case 'Parallel': return { items_path: '', item_alias: 'item', concurrency: 10, mode: 'Rolling', max_failures: null, task_template: null }
    case 'ForkJoin': return { concurrency: 5, mode: 'Rolling', max_failures: null, tasksJson: '[]' }
    default: return {}
  }
}

function buildFormFields(formDef: FormField[]): EditorFormField[] {
  return formDef.map(f => ({
    key: f.key,
    value: f.value ?? '',
    type: f.type || 'String',
    originalType: f.type || 'String',
    description: f.description || '',
  }))
}

function onTaskSelected(node: Node) {
  const task = taskCache.value.find(t => t.id === node.data.taskId)
  if (!task) {
    node.data.taskSnapshot = null
    node.data.formFields = []
    return
  }
  node.data.taskSnapshot = task.task_template
  const tpl = task.task_template as any
  const typeKey = node.data.nodeType
  const inner = tpl[typeKey]
  if (inner?.form) {
    node.data.formFields = buildFormFields(inner.form)
  } else {
    node.data.formFields = []
  }
}

async function onSubWorkflowMetaSelected(node: Node) {
  const metaId = node.data.subWorkflowMetaId
  if (!metaId) return
  const meta = workflowMetas.value.find(m => m.workflow_meta_id === metaId)
  node.data.subWorkflowMeta = meta || null
  try {
    const res = await workflowApi.listTemplates(metaId)
    node.data.subWorkflowVersions = res.data
    if (res.data.length > 0) {
      node.data.subWorkflowVersion = res.data[res.data.length - 1].version
      onSubWorkflowVersionSelected(node)
    }
  } catch {
    node.data.subWorkflowVersions = []
  }
  if (meta?.form?.length) {
    node.data.formFields = buildFormFields(meta.form)
  } else {
    node.data.formFields = []
  }
}

function onSubWorkflowVersionSelected(_node: Node) {
  // version selected, no extra action needed since form is on meta level
}

// ---- Drag & Drop ----

function onDragStart(event: DragEvent, nt: { type: string; label: string }) {
  event.dataTransfer?.setData('nodeType', nt.type)
}

const { screenToFlowCoordinate } = useVueFlow()

function onDrop(event: DragEvent) {
  const type = event.dataTransfer?.getData('nodeType')
  if (!type) return
  const position = screenToFlowCoordinate({ x: event.clientX, y: event.clientY })
  const id = `node_${++nodeCounter}`
  const color = nodeTypes.find(n => n.type === type)?.color || '#86909C'
  nodes.value.push({
    id,
    position,
    data: {
      label: `${id} (${type})`,
      nodeType: type,
      config: getDefaultConfig(type),
      contextJson: '{}',
      taskId: null,
      taskSnapshot: null,
      formFields: [],
      subWorkflowMetaId: null,
      subWorkflowVersion: null,
      subWorkflowMeta: null,
      subWorkflowVersions: [],
    },
    style: { background: color, color: '#fff', borderRadius: '6px', padding: '6px 12px', fontSize: '12px', border: 'none' },
  })
}

function onNodeClick({ node }: { node: Node }) {
  selectedNode.value = node
}

// ---- Build & Save ----

function formFieldsToFormArray(fields: EditorFormField[]): FormField[] {
  return fields
    .filter(f => f.key.trim() !== '')
    .map(f => {
      const field: FormField = { key: f.key, value: f.value, type: f.type as any }
      if (f.description) field.description = f.description
      return field
    })
}

function buildWorkflowEntity(): any {
  const edgeMap = new Map<string, string>()
  for (const e of edges.value) {
    edgeMap.set(e.source, e.target)
  }
  const workflowNodes: any[] = (nodes.value as any[]).map((n: any) => {
    const d = n.data as any
    let config: any
    const taskId = d.taskId || null

    if (isTaskRefNode(d.nodeType) && d.taskSnapshot) {
      config = d.taskSnapshot
    } else if (d.nodeType === 'SubWorkflow') {
      config = {
        SubWorkflow: {
          workflow_meta_id: d.subWorkflowMetaId || '',
          workflow_version: d.subWorkflowVersion || 1,
          form: formFieldsToFormArray(d.formFields || []),
          timeout: d.config.timeout || null,
        },
      }
    } else {
      switch (d.nodeType) {
        case 'IfCondition':
          config = { IfCondition: { name: d.config.name, condition: d.config.condition, then_task: d.config.then_task || null, else_task: d.config.else_task || null } }
          break
        case 'ContextRewrite':
          config = { ContextRewrite: { name: d.config.name, script: d.config.script, merge_mode: d.config.merge_mode || 'Merge' } }
          break
        case 'Parallel':
          config = { Parallel: { items_path: d.config.items_path, item_alias: d.config.item_alias, task_template: d.config.task_template || { Http: { url: '', method: 'Get', headers: [], body: [], form: [], retry_count: 0, retry_delay: 0, timeout: 30, success_condition: null } }, concurrency: d.config.concurrency || 10, mode: d.config.mode || 'Rolling', max_failures: d.config.max_failures } }
          break
        case 'ForkJoin': {
          let tasks: any[] = []
          try { tasks = JSON.parse(d.config.tasksJson || '[]') } catch {}
          config = { ForkJoin: { tasks, concurrency: d.config.concurrency || 5, mode: d.config.mode || 'Rolling', max_failures: d.config.max_failures } }
          break
        }
        default:
          config = d.nodeType
      }
    }

    if (isTaskRefNode(d.nodeType) && d.formFields?.length) {
      const formData = formFieldsToFormArray(d.formFields)
      const typeKey = d.nodeType as string
      if (config && config[typeKey]) {
        config[typeKey] = { ...config[typeKey], form: formData }
      }
    }

    let context: any = {}
    try { context = JSON.parse(d.contextJson || '{}') } catch {}
    return {
      node_id: n.id,
      node_type: d.nodeType,
      task_id: taskId,
      config,
      context,
      next_node: edgeMap.get(n.id) || null,
    }
  })
  return {
    workflow_meta_id: metaId,
    version: currentVersion.value || 1,
    status: 'Draft',
    nodes: workflowNodes,
  }
}

async function handleSave() {
  saving.value = true
  try {
    const entity = buildWorkflowEntity()
    await workflowApi.saveTemplate(metaId, entity)
    currentVersion.value = entity.version
    Notification.success({ content: '保存成功' })
  } catch {} finally { saving.value = false }
}

// ---- Load from entity ----

function loadFromEntity(entity: any) {
  const g = new dagre.graphlib.Graph()
  g.setGraph({ rankdir: 'LR', nodesep: 60, ranksep: 100 })
  g.setDefaultEdgeLabel(() => ({}))
  for (const n of entity.nodes) {
    g.setNode(n.node_id, { width: 160, height: 44 })
    if (n.next_node) g.setEdge(n.node_id, n.next_node)
  }
  dagre.layout(g)
  nodeCounter = entity.nodes.length
  nodes.value = entity.nodes.map((n: any) => {
    const pos = g.node(n.node_id)
    const type = n.node_type
    const color = nodeTypes.find(t => t.type === type)?.color || '#86909C'
    let config: any = getDefaultConfig(type)
    let taskId: string | null = n.task_id || null
    let taskSnapshot: any = null
    let formFields: EditorFormField[] = []
    let subWorkflowMetaId: string | null = null
    let subWorkflowVersion: number | null = null
    let subWorkflowMeta: any = null

    if (isTaskRefNode(type) && n.config) {
      taskSnapshot = n.config
      if (taskId) {
        const task = taskCache.value.find(t => t.id === taskId)
        if (task) {
          const inner = (n.config as any)[type]
          if (inner?.form) {
            formFields = buildFormFields(inner.form)
          }
        }
      }
      const inner = (n.config as any)[type]
      if (inner?.form && formFields.length === 0) {
        formFields = buildFormFields(inner.form)
      }
    } else if (type === 'SubWorkflow' && n.config?.SubWorkflow) {
      const s = n.config.SubWorkflow
      subWorkflowMetaId = s.workflow_meta_id
      subWorkflowVersion = s.workflow_version
      config = { timeout: s.timeout }
      if (s.form?.length) {
        formFields = buildFormFields(s.form)
      }
      const meta = workflowMetas.value.find(m => m.workflow_meta_id === subWorkflowMetaId)
      subWorkflowMeta = meta || null
    } else if (type === 'IfCondition' && n.config?.IfCondition) {
      config = { ...n.config.IfCondition }
    } else if (type === 'ContextRewrite' && n.config?.ContextRewrite) {
      config = { ...n.config.ContextRewrite }
    } else if (type === 'Parallel' && n.config?.Parallel) {
      const p = n.config.Parallel
      config = { items_path: p.items_path, item_alias: p.item_alias, concurrency: p.concurrency, mode: p.mode, max_failures: p.max_failures, task_template: p.task_template }
    } else if (type === 'ForkJoin' && n.config?.ForkJoin) {
      const f = n.config.ForkJoin
      config = { concurrency: f.concurrency, mode: f.mode, max_failures: f.max_failures, tasksJson: JSON.stringify(f.tasks, null, 2) }
    }

    return {
      id: n.node_id,
      position: { x: pos?.x || 0, y: pos?.y || 0 },
      data: {
        label: `${n.node_id} (${type})`,
        nodeType: type,
        config,
        contextJson: JSON.stringify(n.context || {}, null, 2),
        taskId,
        taskSnapshot,
        formFields,
        subWorkflowMetaId,
        subWorkflowVersion,
        subWorkflowMeta,
        subWorkflowVersions: [],
      },
      style: { background: color, color: '#fff', borderRadius: '6px', padding: '6px 12px', fontSize: '12px', border: 'none' },
    }
  })
  edges.value = entity.nodes
    .filter((n: any) => n.next_node)
    .map((n: any) => ({ id: `${n.node_id}->${n.next_node}`, source: n.node_id, target: n.next_node, type: 'smoothstep' }))
}

// ---- Init ----

onMounted(async () => {
  try {
    const [metaRes, tasksRes, metasRes] = await Promise.all([
      workflowApi.getMeta(metaId),
      taskApi.list(),
      workflowApi.listMeta(),
    ])
    metaName.value = metaRes.data.name
    taskCache.value = tasksRes.data
    workflowMetas.value = metasRes.data
  } catch {}

  if (versionParam) {
    try {
      const res = await workflowApi.getTemplate(metaId, versionParam)
      loadFromEntity(res.data)
    } catch {}
  }
})
</script>

<style scoped>
.editor-container {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 88px);
}
.editor-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-2);
}
.editor-title {
  flex: 1;
  font-size: 16px;
  font-weight: 600;
}
.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}
.editor-panel-left {
  width: 180px;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-2);
  padding: 12px;
  overflow-y: auto;
}
.editor-canvas {
  flex: 1;
  position: relative;
}
.editor-panel-right {
  width: 320px;
  border-left: 1px solid var(--color-border);
  background: var(--color-bg-2);
  padding: 12px;
  overflow-y: auto;
}
.panel-title {
  font-weight: 600;
  margin-bottom: 12px;
  font-size: 14px;
}
.node-item {
  padding: 8px 12px;
  margin-bottom: 6px;
  border-radius: 4px;
  border-left: 3px solid;
  background: var(--color-fill-2);
  cursor: grab;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.node-item:hover {
  background: var(--color-fill-3);
}
.node-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
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

<style>
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
@import '@vue-flow/controls/dist/style.css';
@import '@vue-flow/minimap/dist/style.css';
</style>
