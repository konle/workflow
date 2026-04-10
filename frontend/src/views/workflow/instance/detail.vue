<template>
  <div>
    <a-page-header :title="`工作流实例: ${instance?.workflow_instance_id || ''}`" @back="$router.push('/workflows/instances')">
      <template #extra>
        <a-space>
          <a-button v-if="instance?.status === 'Pending' && canExecute" type="primary" @click="handleExecute">执行</a-button>
          <a-button v-if="instance?.status === 'Failed' && canExecute" @click="handleRetry">重试</a-button>
          <a-button v-if="instance?.status === 'Suspended' && canExecute" @click="handleResume">恢复</a-button>
          <a-button v-if="['Failed','Suspended'].includes(instance?.status || '') && canExecute" status="danger" @click="handleCancel">取消</a-button>
          <a-button v-if="canSkipNode" @click="openSkipModal">跳过当前节点</a-button>
        </a-space>
      </template>
    </a-page-header>

    <a-modal v-model:visible="skipModalVisible" title="跳过节点" @ok="submitSkip" @cancel="closeSkipModal">
      <p class="skip-hint">须为当前失败/挂起节点填写 <code>output</code> JSON 对象（可为空对象 <code>{}</code>），供下游 <code>nodes.&lt;node_id&gt;.output</code> 引用。提交后将投递编排回调，一般无需再点「执行」。</p>
      <a-textarea v-model="skipOutputText" :auto-size="{ minRows: 6, maxRows: 16 }" placeholder="{}" />
    </a-modal>

    <a-row :gutter="16">
      <a-col :span="6">
        <a-card title="基本信息" :loading="loading" size="small">
          <a-descriptions :column="1" size="small">
            <a-descriptions-item label="状态">
              <status-tag v-if="instance" :status="instance.status" :map="WORKFLOW_INSTANCE_STATUS_MAP" />
            </a-descriptions-item>
            <a-descriptions-item label="版本">{{ instance?.workflow_version }}</a-descriptions-item>
            <a-descriptions-item label="当前节点">{{ instance?.current_node || '-' }}</a-descriptions-item>
            <a-descriptions-item label="创建时间">{{ formatDate(instance?.created_at || '') }}</a-descriptions-item>
          </a-descriptions>
          <a-divider>Context</a-divider>
          <json-viewer :data="instance?.context" />
        </a-card>
      </a-col>

      <a-col :span="12">
        <a-card title="节点拓扑" size="small" style="height: 500px">
          <div ref="flowContainer" style="width: 100%; height: 420px">
            <VueFlow
              v-if="flowNodes.length"
              :nodes="flowNodes"
              :edges="flowEdges"
              :nodes-draggable="false"
              :nodes-connectable="false"
              fit-view-on-init
              @node-click="({ node }: any) => handleNodeClick(node.id)"
            >
              <Background />
            </VueFlow>
          </div>
        </a-card>
      </a-col>

      <a-col :span="6">
        <a-card title="节点详情" size="small">
          <template v-if="selectedNode">
            <a-descriptions :column="1" size="small">
              <a-descriptions-item label="节点ID">{{ selectedNode.node_id }}</a-descriptions-item>
              <a-descriptions-item label="类型">{{ selectedNode.node_type }}</a-descriptions-item>
              <a-descriptions-item label="状态">
                <status-tag :status="selectedNode.status" :map="NODE_STATUS_MAP" />
              </a-descriptions-item>
              <a-descriptions-item v-if="selectedNode.task_instance?.task_id" label="任务模板ID">
                <a-typography-text copyable>{{ selectedNode.task_instance.task_id }}</a-typography-text>
              </a-descriptions-item>
            </a-descriptions>
            <a-divider>节点上下文</a-divider>
            <p class="node-context-hint">
              执行本节点前用于模板 / Rhai 解析的合并上下文（含变量合并与系统注入的 <code>nodes</code>）。
            </p>
            <json-viewer :data="selectedNode.context ?? {}" />
            <a-divider>Input</a-divider>
            <json-viewer v-if="selectedNode.task_instance?.input != null" :data="selectedNode.task_instance.input" />
            <a-empty v-else description="暂无执行入参" />
            <a-divider>Output</a-divider>
            <json-viewer v-if="selectedNode.task_instance?.output != null" :data="selectedNode.task_instance.output" />
            <a-empty v-else description="暂无输出" />
            <template v-if="selectedNode.error_message">
              <a-divider>Error</a-divider>
              <a-alert type="error" :title="selectedNode.error_message" />
            </template>
          </template>
          <a-empty v-else description="点击节点查看详情" />
        </a-card>
      </a-col>
    </a-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { VueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { workflowApi } from '../../../api/workflow'
import { usePermission } from '../../../composables/use-permission'
import { usePolling } from '../../../composables/use-polling'
import { WORKFLOW_INSTANCE_STATUS_MAP, NODE_STATUS_MAP } from '../../../utils/constants'
import { formatDate } from '../../../utils/format'
import StatusTag from '../../../components/common/status-tag.vue'
import JsonViewer from '../../../components/common/json-viewer.vue'
import { Notification } from '@arco-design/web-vue'
import type { WorkflowInstanceEntity, WorkflowNodeInstanceEntity } from '../../../types/workflow'
import dagre from 'dagre'

const route = useRoute()
const { canExecute } = usePermission()
const instanceId = route.params.id as string

const instance = ref<WorkflowInstanceEntity | null>(null)
const loading = ref(false)
const selectedNode = ref<WorkflowNodeInstanceEntity | null>(null)
const skipModalVisible = ref(false)
const skipOutputText = ref('{}')

const UNSKIPPABLE_TYPES = new Set(['Parallel', 'ForkJoin', 'SubWorkflow'])

const canSkipNode = computed(() => {
  if (!canExecute.value || !instance.value) return false
  const st = instance.value.status
  if (st !== 'Failed' && st !== 'Suspended') return false
  const cur = instance.value.current_node
  const n = instance.value.nodes.find(x => x.node_id === cur)
  if (!n) return false
  if (n.status !== 'Failed' && n.status !== 'Suspended') return false
  if (UNSKIPPABLE_TYPES.has(n.node_type)) return false
  return true
})

const NODE_COLORS: Record<string, string> = {
  Pending: '#C9CDD4',
  Running: '#3491FA',
  Success: '#00B42A',
  Failed: '#F53F3F',
  Suspended: '#F77234',
  Skipped: '#E5E6EB',
}

function collectEdges(nodes: WorkflowNodeInstanceEntity[]) {
  const edges: { source: string; target: string; label?: string; branch?: boolean }[] = []
  for (const n of nodes) {
    if (n.next_node) {
      edges.push({ source: n.node_id, target: n.next_node })
    }
    if (n.node_type === 'IfCondition') {
      const tpl = (n.task_instance.task_template as { IfCondition: { then_task?: string | null; else_task?: string | null } }).IfCondition
      if (tpl?.then_task) edges.push({ source: n.node_id, target: tpl.then_task, label: 'Y', branch: true })
      if (tpl?.else_task) edges.push({ source: n.node_id, target: tpl.else_task, label: 'N', branch: true })
    }
  }
  const seen = new Set<string>()
  return edges.filter(e => {
    const key = `${e.source}->${e.target}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

const flowNodes = computed(() => {
  if (!instance.value) return []
  const g = new dagre.graphlib.Graph()
  g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 80 })
  g.setDefaultEdgeLabel(() => ({}))
  for (const n of instance.value.nodes) {
    g.setNode(n.node_id, { width: 150, height: 40 })
  }
  for (const e of collectEdges(instance.value.nodes)) {
    g.setEdge(e.source, e.target)
  }
  dagre.layout(g)
  return instance.value.nodes.map(n => {
    const pos = g.node(n.node_id)
    return {
      id: n.node_id,
      position: { x: pos?.x || 0, y: pos?.y || 0 },
      data: { label: `${n.node_id} (${n.node_type})` },
      style: {
        background: NODE_COLORS[n.status] || '#C9CDD4',
        color: ['Running', 'Failed', 'Success'].includes(n.status) ? '#fff' : '#1D2129',
        borderRadius: '6px',
        padding: '6px 12px',
        fontSize: '12px',
        border: 'none',
        cursor: 'pointer',
      },
    }
  })
})

const flowEdges = computed(() => {
  if (!instance.value) return []
  return collectEdges(instance.value.nodes).map(e => ({
    id: `${e.source}->${e.target}`,
    source: e.source,
    target: e.target,
    animated: instance.value!.nodes.find(n => n.node_id === e.source)?.status === 'Running',
    label: e.label,
    style: e.branch ? { strokeDasharray: '5 5' } : {},
  }))
})

async function fetchInstance() {
  try {
    const res = await workflowApi.getInstance(instanceId)
    instance.value = res.data
  } catch {}
}

const { start: startPolling, stop: stopPolling } = usePolling(fetchInstance, 5000)

watch(() => instance.value?.status, (status) => {
  if (status && ['Completed', 'Canceled'].includes(status)) {
    stopPolling()
  }
})

function handleNodeClick(nodeId: string) {
  selectedNode.value = instance.value?.nodes.find(n => n.node_id === nodeId) || null
}

async function handleExecute() {
  await workflowApi.executeInstance(instanceId)
  Notification.success({ content: '已执行' })
  fetchInstance()
}

async function handleRetry() {
  await workflowApi.retryInstance(instanceId)
  Notification.success({ content: '已重试' })
  fetchInstance()
}

async function handleResume() {
  await workflowApi.resumeInstance(instanceId)
  Notification.success({ content: '已恢复' })
  fetchInstance()
}

async function handleCancel() {
  await workflowApi.cancelInstance(instanceId)
  Notification.success({ content: '已取消' })
  fetchInstance()
}


function openSkipModal() {
  skipOutputText.value = '{}'
  skipModalVisible.value = true
}

function closeSkipModal() {
  skipModalVisible.value = false
}

async function submitSkip() {
  if (!instance.value) return
  let output: Record<string, unknown>
  try {
    const parsed = JSON.parse(skipOutputText.value || '{}')
    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
      Notification.error({ content: 'output 须为 JSON 对象' })
      return
    }
    output = parsed as Record<string, unknown>
  } catch {
    Notification.error({ content: 'output 不是合法 JSON' })
    return
  }
  const nodeId = instance.value.current_node
  try {
    await workflowApi.skipNode(instanceId, { node_id: nodeId, output })
    Notification.success({ content: '已跳过并投递编排' })
    skipModalVisible.value = false
    fetchInstance()
  } catch {
    /* axios 拦截器已提示 */
  }
}


onMounted(async () => {
  loading.value = true
  await fetchInstance()
  loading.value = false
  if (instance.value && !['Completed', 'Canceled'].includes(instance.value.status)) {
    startPolling()
  }
})
</script>

<style scoped>
.node-context-hint {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--color-text-3);
  line-height: 1.5;
}
.node-context-hint code {
  font-size: 11px;
  padding: 0 4px;
  border-radius: 2px;
  background: var(--color-fill-2);
}

.skip-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--color-text-3);
  line-height: 1.5;
}
.skip-hint code {
  font-size: 11px;
  padding: 0 4px;
  border-radius: 2px;
  background: var(--color-fill-2);
}
</style>

<style>
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
</style>
