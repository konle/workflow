<template>
  <div>
    <a-page-header title="审批中心" :show-back="false" />
    <a-card>
      <a-tabs v-model:active-key="activeTab" @change="onTabChange">
        <a-tab-pane key="mine" title="我的待办">
          <a-table :data="mineList" :columns="columns" :loading="loading" row-key="id">
            <template #status="{ record }">
              <status-tag :status="record.status" :map="APPROVAL_STATUS_MAP" />
            </template>
            <template #approval_mode="{ record }">
              <status-tag :status="record.approval_mode" :map="APPROVAL_MODE_MAP" />
            </template>
            <template #progress="{ record }">
              {{ record.decisions.length }} / {{ record.approvers.length }}
            </template>
            <template #workflow="{ record }">
              <a-link @click="$router.push(`/workflows/instances/${record.workflow_instance_id}`)">
                {{ record.workflow_instance_id.slice(0, 8) }}...
              </a-link>
            </template>
            <template #created_at="{ record }">
              {{ record.created_at?.slice(0, 19).replace('T', ' ') }}
            </template>
            <template #actions="{ record }">
              <a-button type="text" size="small" @click="$router.push(`/approvals/${record.id}`)">
                去审批
              </a-button>
            </template>
          </a-table>
        </a-tab-pane>
        <a-tab-pane v-if="canManageUsers || isSuperAdmin" key="all" title="全部审批">
          <a-table :data="allList" :columns="columns" :loading="loading" row-key="id">
            <template #status="{ record }">
              <status-tag :status="record.status" :map="APPROVAL_STATUS_MAP" />
            </template>
            <template #approval_mode="{ record }">
              <status-tag :status="record.approval_mode" :map="APPROVAL_MODE_MAP" />
            </template>
            <template #progress="{ record }">
              {{ record.decisions.length }} / {{ record.approvers.length }}
            </template>
            <template #workflow="{ record }">
              <a-link @click="$router.push(`/workflows/instances/${record.workflow_instance_id}`)">
                {{ record.workflow_instance_id.slice(0, 8) }}...
              </a-link>
            </template>
            <template #created_at="{ record }">
              {{ record.created_at?.slice(0, 19).replace('T', ' ') }}
            </template>
            <template #actions="{ record }">
              <a-button type="text" size="small" @click="$router.push(`/approvals/${record.id}`)">
                查看
              </a-button>
            </template>
          </a-table>
        </a-tab-pane>
      </a-tabs>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { approvalApi } from '../../api/approval'
import { usePermission } from '../../composables/use-permission'
import { APPROVAL_STATUS_MAP, APPROVAL_MODE_MAP } from '../../utils/constants'
import StatusTag from '../../components/common/status-tag.vue'
import type { ApprovalInstanceEntity } from '../../types/approval'

const { canManageUsers, isSuperAdmin } = usePermission()

const activeTab = ref('mine')
const loading = ref(false)
const mineList = ref<ApprovalInstanceEntity[]>([])
const allList = ref<ApprovalInstanceEntity[]>([])

const columns = [
  { title: '审批标题', dataIndex: 'title', ellipsis: true },
  { title: '审批模式', slotName: 'approval_mode', width: 120 },
  { title: '状态', slotName: 'status', width: 100 },
  { title: '进度', slotName: 'progress', width: 80 },
  { title: '关联工作流', slotName: 'workflow', width: 140 },
  { title: '创建时间', slotName: 'created_at', width: 170 },
  { title: '操作', slotName: 'actions', width: 90 },
]

async function loadMine() {
  loading.value = true
  try {
    const res = await approvalApi.listMine()
    mineList.value = res.data
  } catch {} finally { loading.value = false }
}

async function loadAll() {
  loading.value = true
  try {
    const res = await approvalApi.listAll()
    allList.value = res.data
  } catch {} finally { loading.value = false }
}

function onTabChange(key: string | number) {
  if (key === 'mine') loadMine()
  else if (key === 'all') loadAll()
}

onMounted(() => loadMine())
</script>
