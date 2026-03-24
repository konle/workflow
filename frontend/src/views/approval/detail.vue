<template>
  <div>
    <a-page-header title="审批详情" @back="$router.push('/approvals')" />
    <a-spin :loading="loading" style="width: 100%">
      <template v-if="approval">
        <a-row :gutter="16">
          <a-col :span="16">
            <a-card title="基础信息">
              <a-descriptions :column="2" bordered>
                <a-descriptions-item label="审批标题">{{ approval.title }}</a-descriptions-item>
                <a-descriptions-item label="状态">
                  <status-tag :status="approval.status" :map="APPROVAL_STATUS_MAP" />
                </a-descriptions-item>
                <a-descriptions-item label="审批模式">
                  <status-tag :status="approval.approval_mode" :map="APPROVAL_MODE_MAP" />
                </a-descriptions-item>
                <a-descriptions-item label="进度">
                  {{ approval.decisions.length }} / {{ approval.approvers.length }}
                </a-descriptions-item>
                <a-descriptions-item label="关联工作流" :span="2">
                  <a-link @click="$router.push(`/workflows/instances/${approval.workflow_instance_id}`)">
                    {{ approval.workflow_instance_id }}
                  </a-link>
                </a-descriptions-item>
                <a-descriptions-item v-if="approval.description" label="描述" :span="2">
                  {{ approval.description }}
                </a-descriptions-item>
                <a-descriptions-item label="创建时间">
                  {{ approval.created_at?.slice(0, 19).replace('T', ' ') }}
                </a-descriptions-item>
                <a-descriptions-item label="过期时间">
                  {{ approval.expires_at ? approval.expires_at.slice(0, 19).replace('T', ' ') : '无' }}
                </a-descriptions-item>
              </a-descriptions>
            </a-card>

            <a-card title="决策记录" style="margin-top: 16px">
              <a-timeline v-if="approval.decisions.length > 0">
                <a-timeline-item
                  v-for="d in approval.decisions"
                  :key="d.user_id + d.decided_at"
                  :dot-color="d.decision === 'Approve' ? '#00B42A' : '#F53F3F'"
                >
                  <div class="decision-item">
                    <span class="decision-user">{{ d.user_id.slice(0, 8) }}...</span>
                    <a-tag :color="d.decision === 'Approve' ? 'green' : 'red'" size="small">
                      {{ d.decision === 'Approve' ? '通过' : '拒绝' }}
                    </a-tag>
                    <span class="decision-time">{{ d.decided_at?.slice(0, 19).replace('T', ' ') }}</span>
                  </div>
                  <div v-if="d.comment" class="decision-comment">{{ d.comment }}</div>
                </a-timeline-item>
              </a-timeline>
              <a-empty v-else description="暂无决策记录" />
            </a-card>
          </a-col>

          <a-col :span="8">
            <a-card title="审批人">
              <div v-for="uid in approval.approvers" :key="uid" class="approver-row">
                <span class="approver-id">{{ uid.slice(0, 12) }}...</span>
                <a-tag v-if="getDecision(uid)" :color="getDecision(uid)!.decision === 'Approve' ? 'green' : 'red'" size="small">
                  {{ getDecision(uid)!.decision === 'Approve' ? '已通过' : '已拒绝' }}
                </a-tag>
                <a-tag v-else color="orange" size="small">待审批</a-tag>
              </div>
            </a-card>

            <a-card v-if="canDecide" title="提交决策" style="margin-top: 16px">
              <a-form layout="vertical">
                <a-form-item label="评论（可选）">
                  <a-textarea v-model="comment" :auto-size="{ minRows: 2 }" placeholder="输入评论..." />
                </a-form-item>
                <a-space>
                  <a-button type="primary" status="success" :loading="deciding" @click="handleDecide('Approve')">
                    通过
                  </a-button>
                  <a-button type="primary" status="danger" :loading="deciding" @click="handleDecide('Reject')">
                    拒绝
                  </a-button>
                </a-space>
              </a-form>
            </a-card>
          </a-col>
        </a-row>
      </template>
    </a-spin>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { approvalApi } from '../../api/approval'
import { useAuthStore } from '../../stores/auth'
import { APPROVAL_STATUS_MAP, APPROVAL_MODE_MAP } from '../../utils/constants'
import StatusTag from '../../components/common/status-tag.vue'
import { Notification } from '@arco-design/web-vue'
import type { ApprovalInstanceEntity, ApprovalDecision, Decision } from '../../types/approval'

const route = useRoute()
const auth = useAuthStore()
const approvalId = route.params.id as string

const loading = ref(false)
const deciding = ref(false)
const approval = ref<ApprovalInstanceEntity | null>(null)
const comment = ref('')

const canDecide = computed(() => {
  if (!approval.value) return false
  if (approval.value.status !== 'Pending') return false
  if (!approval.value.approvers.includes(auth.userId)) return false
  if (approval.value.decisions.some(d => d.user_id === auth.userId)) return false
  return true
})

function getDecision(userId: string): ApprovalDecision | undefined {
  return approval.value?.decisions.find(d => d.user_id === userId)
}

async function loadApproval() {
  loading.value = true
  try {
    const res = await approvalApi.get(approvalId)
    approval.value = res.data
  } catch {} finally { loading.value = false }
}

async function handleDecide(decision: Decision) {
  deciding.value = true
  try {
    const res = await approvalApi.decide(approvalId, {
      decision,
      comment: comment.value || undefined,
    })
    approval.value = res.data
    comment.value = ''
    Notification.success({ content: decision === 'Approve' ? '已通过' : '已拒绝' })
  } catch {} finally { deciding.value = false }
}

onMounted(() => loadApproval())
</script>

<style scoped>
.approver-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--color-border);
}
.approver-row:last-child {
  border-bottom: none;
}
.approver-id {
  font-family: monospace;
  font-size: 13px;
}
.decision-item {
  display: flex;
  align-items: center;
  gap: 8px;
}
.decision-user {
  font-family: monospace;
  font-size: 13px;
}
.decision-time {
  color: var(--color-text-3);
  font-size: 12px;
}
.decision-comment {
  margin-top: 4px;
  padding: 6px 10px;
  background: var(--color-fill-2);
  border-radius: 4px;
  font-size: 13px;
  color: var(--color-text-2);
}
</style>
