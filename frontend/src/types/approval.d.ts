export type ApprovalStatus = 'Pending' | 'Approved' | 'Rejected'
export type ApprovalMode = 'Any' | 'All' | 'Majority'
export type Decision = 'Approve' | 'Reject'

export interface ApproverRule {
  User?: string
  Role?: string
  ContextVariable?: string
}

export type SelfApprovalPolicy = 'Allow' | 'Skip'

export interface ApprovalTemplate {
  name: string
  title: string
  description: string | null
  approvers: ApproverRule[]
  approval_mode: ApprovalMode
  timeout: number | null
  self_approval?: SelfApprovalPolicy
}

export interface ApprovalDecision {
  user_id: string
  decision: Decision
  comment: string | null
  decided_at: string
}

export interface ApprovalInstanceEntity {
  id: string
  tenant_id: string
  workflow_instance_id: string
  node_id: string
  title: string
  description: string | null
  approval_mode: ApprovalMode
  approvers: string[]
  decisions: ApprovalDecision[]
  status: ApprovalStatus
  created_at: string
  updated_at: string
  expires_at: string | null
  applicant_id: string | null
}
