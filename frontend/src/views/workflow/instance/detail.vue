<template>
  <div>
    <a-page-header :title="`工作流实例: ${instance?.workflow_instance_id || ''}`" @back="$router.push('/workflows/instances')">
      <template #extra>
        <a-space>
          <a-button v-if="instance?.status === 'Pending' && canExecute" type="primary" @click="handleExecute">执行</a-button>
          <a-button v-if="instance?.status === 'Failed' && canExecute" @click="handleRetry">重试</a-button>
          <a-button v-if="instance?.status === 'Suspended' && canExecute" @click="handleResume">恢复</a-button>
          <a-button v-if="['Failed','Suspended'].includes(instance?.status || '') && canExecute" status="danger" @click="handleCancel">取消</a-button>
        </a-space>
      </template>
    </a-page-header>

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
            </a-descriptions>
            <a-divider>Output</a-divider>
            <json-viewer :data="selectedNode.output?.data" />
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

const NODE_COLORS: Record<string, string> = {
  Pending: '#C9CDD4',
  Running: '#3491FA',
  Success: '#00B42A',
  Failed: '#F53F3F',
  Suspended: '#F77234',
  Skipped: '#E5E6EB',
}

const flowNodes = computed(() => {
  if (!instance.value) return []
  const g = new dagre.graphlib.Graph()
  g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 80 })
  g.setDefaultEdgeLabel(() => ({}))
  for (const n of instance.value.nodes) {
    g.setNode(n.node_id, { width: 150, height: 40 })
    if (n.next_node) g.setEdge(n.node_id, n.next_node)
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
  const edges: any[] = []
  for (const n of instance.value.nodes) {
    if (n.next_node) {
      edges.push({ id: `${n.node_id}->${n.next_node}`, source: n.node_id, target: n.next_node, animated: n.status === 'Running' })
    }
  }
  return edges
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

onMounted(async () => {
  loading.value = true
  await fetchInstance()
  loading.value = false
  if (instance.value && !['Completed', 'Canceled'].includes(instance.value.status)) {
    startPolling()
  }
})
</script>

<style>
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
</style>
