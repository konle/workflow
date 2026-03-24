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

            <template v-if="selectedNode.data.nodeType === 'Http'">
              <a-form-item label="URL">
                <a-input v-model="selectedNode.data.config.url" />
              </a-form-item>
              <a-form-item label="Method">
                <a-select v-model="selectedNode.data.config.method">
                  <a-option value="Get">GET</a-option>
                  <a-option value="Post">POST</a-option>
                  <a-option value="Put">PUT</a-option>
                  <a-option value="Delete">DELETE</a-option>
                </a-select>
              </a-form-item>
              <a-form-item label="Headers (JSON)">
                <a-textarea v-model="selectedNode.data.config.headersJson" :auto-size="{ minRows: 2 }" />
              </a-form-item>
              <a-form-item label="Body">
                <a-textarea v-model="selectedNode.data.config.bodyJson" :auto-size="{ minRows: 2 }" />
              </a-form-item>
              <a-form-item label="超时(秒)">
                <a-input-number v-model="selectedNode.data.config.timeout" :min="1" />
              </a-form-item>
              <a-form-item label="重试次数">
                <a-input-number v-model="selectedNode.data.config.retry_count" :min="0" />
              </a-form-item>
            </template>

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

            <template v-else-if="selectedNode.data.nodeType === 'SubWorkflow'">
              <a-form-item label="工作流 Meta ID">
                <a-input v-model="selectedNode.data.config.workflow_meta_id" />
              </a-form-item>
              <a-form-item label="版本">
                <a-input-number v-model="selectedNode.data.config.workflow_version" :min="1" />
              </a-form-item>
              <a-form-item label="Input Mapping (JSON)">
                <a-textarea v-model="selectedNode.data.config.inputMappingJson" :auto-size="{ minRows: 2 }" />
              </a-form-item>
              <a-form-item label="超时(秒)">
                <a-input-number v-model="selectedNode.data.config.timeout" />
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
import { TASK_TYPE_MAP } from '../../../utils/constants'
import { Notification } from '@arco-design/web-vue'
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

let nodeCounter = 0

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

function getDefaultConfig(type: string): any {
  switch (type) {
    case 'Http': return { url: '', method: 'Get', headersJson: '{}', bodyJson: '', timeout: 30, retry_count: 0, retry_delay: 0, success_condition: '' }
    case 'IfCondition': return { name: '', condition: '', then_task: '', else_task: '' }
    case 'ContextRewrite': return { name: '', script: '', merge_mode: 'Merge' }
    case 'Parallel': return { items_path: '', item_alias: 'item', concurrency: 10, mode: 'Rolling', max_failures: null, task_template: null }
    case 'ForkJoin': return { concurrency: 5, mode: 'Rolling', max_failures: null, tasksJson: '[]' }
    case 'SubWorkflow': return { workflow_meta_id: '', workflow_version: 1, inputMappingJson: '{}', timeout: null }
    default: return {}
  }
}

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
    data: { label: `${id} (${type})`, nodeType: type, config: getDefaultConfig(type), contextJson: '{}' },
    style: { background: color, color: '#fff', borderRadius: '6px', padding: '6px 12px', fontSize: '12px', border: 'none' },
  })
}

function onNodeClick({ node }: { node: Node }) {
  selectedNode.value = node
}

function buildWorkflowEntity(): any {
  const edgeMap = new Map<string, string>()
  for (const e of edges.value) {
    edgeMap.set(e.source, e.target)
  }
  const workflowNodes = nodes.value.map(n => {
    const d = n.data as any
    let config: any
    switch (d.nodeType) {
      case 'Http': {
        let headers = {}
        try { headers = JSON.parse(d.config.headersJson || '{}') } catch {}
        const bodyVal = (d.config.bodyJson || '').trim()
        config = {
          Http: {
            url: d.config.url || '',
            method: d.config.method || 'Get',
            headers,
            body: bodyVal ? { key: 'body', value: bodyVal, type: 'json' } : null,
            form: null,
            retry_count: d.config.retry_count || 0,
            retry_delay: d.config.retry_delay || 0,
            timeout: d.config.timeout || 30,
            success_condition: d.config.success_condition || null,
          },
        }
        break
      }
      case 'IfCondition':
        config = { IfCondition: { name: d.config.name, condition: d.config.condition, then_task: d.config.then_task || null, else_task: d.config.else_task || null } }
        break
      case 'ContextRewrite':
        config = { ContextRewrite: { name: d.config.name, script: d.config.script, merge_mode: d.config.merge_mode || 'Merge' } }
        break
      case 'Parallel':
        config = { Parallel: { items_path: d.config.items_path, item_alias: d.config.item_alias, task_template: d.config.task_template || { Http: { url: '', method: 'Get', headers: {}, body: null, form: null, retry_count: 0, retry_delay: 0, timeout: 30, success_condition: null } }, concurrency: d.config.concurrency || 10, mode: d.config.mode || 'Rolling', max_failures: d.config.max_failures } }
        break
      case 'ForkJoin': {
        let tasks = []
        try { tasks = JSON.parse(d.config.tasksJson || '[]') } catch {}
        config = { ForkJoin: { tasks, concurrency: d.config.concurrency || 5, mode: d.config.mode || 'Rolling', max_failures: d.config.max_failures } }
        break
      }
      case 'SubWorkflow': {
        let inputMapping = null
        try { inputMapping = JSON.parse(d.config.inputMappingJson || 'null') } catch {}
        config = { SubWorkflow: { workflow_meta_id: d.config.workflow_meta_id, workflow_version: d.config.workflow_version, input_mapping: inputMapping, output_path: null, timeout: d.config.timeout } }
        break
      }
      default:
        config = d.nodeType
    }
    let context = {}
    try { context = JSON.parse(d.contextJson || '{}') } catch {}
    return {
      node_id: n.id,
      node_type: d.nodeType,
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
    if (type === 'Http' && n.config?.Http) {
      const h = n.config.Http
      config = { ...config, url: h.url, method: h.method, headersJson: JSON.stringify(h.headers || {}, null, 2), bodyJson: h.body?.value || '', timeout: h.timeout, retry_count: h.retry_count, retry_delay: h.retry_delay, success_condition: h.success_condition || '' }
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
    } else if (type === 'SubWorkflow' && n.config?.SubWorkflow) {
      const s = n.config.SubWorkflow
      config = { workflow_meta_id: s.workflow_meta_id, workflow_version: s.workflow_version, inputMappingJson: JSON.stringify(s.input_mapping, null, 2), timeout: s.timeout }
    }
    return {
      id: n.node_id,
      position: { x: pos?.x || 0, y: pos?.y || 0 },
      data: { label: `${n.node_id} (${type})`, nodeType: type, config, contextJson: JSON.stringify(n.context || {}, null, 2) },
      style: { background: color, color: '#fff', borderRadius: '6px', padding: '6px 12px', fontSize: '12px', border: 'none' },
    }
  })
  edges.value = entity.nodes
    .filter((n: any) => n.next_node)
    .map((n: any) => ({ id: `${n.node_id}->${n.next_node}`, source: n.node_id, target: n.next_node, type: 'smoothstep' }))
}

onMounted(async () => {
  try {
    const metaRes = await workflowApi.getMeta(metaId)
    metaName.value = metaRes.data.name
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
  width: 300px;
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
</style>

<style>
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
@import '@vue-flow/controls/dist/style.css';
@import '@vue-flow/minimap/dist/style.css';
</style>
