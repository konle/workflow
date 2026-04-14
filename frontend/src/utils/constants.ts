export const WORKFLOW_INSTANCE_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Pending: { label: '待执行', color: 'gray' },
  Running: { label: '运行中', color: 'arcoblue' },
  Await: { label: '等待中', color: 'orangered' },
  Completed: { label: '已完成', color: 'green' },
  Failed: { label: '已失败', color: 'red' },
  Canceled: { label: '已取消', color: 'gray' },
  Suspended: { label: '已挂起', color: 'orange' },
}

export const TASK_INSTANCE_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Pending: { label: '待执行', color: 'gray' },
  Running: { label: '运行中', color: 'arcoblue' },
  Completed: { label: '已完成', color: 'green' },
  Failed: { label: '已失败', color: 'red' },
  Canceled: { label: '已取消', color: 'gray' },
  Skipped: { label: '已跳过', color: 'orange' },
}

export const TEMPLATE_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Draft: { label: '草稿', color: 'gray' },
  Published: { label: '已发布', color: 'green' },
}

export const NODE_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Pending: { label: '待执行', color: 'gray' },
  Running: { label: '运行中', color: 'arcoblue' },
  Success: { label: '成功', color: 'green' },
  Failed: { label: '失败', color: 'red' },
  Suspended: { label: '挂起', color: 'orange' },
  Skipped: { label: '已跳过', color: 'gray' },
}

export const TASK_TYPE_MAP: Record<string, { label: string; color: string }> = {
  Http: { label: 'HTTP', color: '#3491FA' },
  Grpc: { label: 'gRPC', color: '#722ED1' },
  Approval: { label: '审批', color: '#F77234' },
  IfCondition: { label: '条件分支', color: '#F7BA1E' },
  ContextRewrite: { label: '上下文重写', color: '#14C9C9' },
  Parallel: { label: '并发容器', color: '#00B42A' },
  ForkJoin: { label: '异构并发', color: '#009A29' },
  SubWorkflow: { label: '子工作流', color: '#86909C' },
  Pause: { label: '暂停', color: '#F5A623' },
}

export const TENANT_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Active: { label: '正常', color: 'green' },
  Suspended: { label: '已暂停', color: 'orange' },
  Deleted: { label: '已删除', color: 'red' },
}

export const APPROVAL_STATUS_MAP: Record<string, { label: string; color: string }> = {
  Pending: { label: '待审批', color: 'orange' },
  Approved: { label: '已通过', color: 'green' },
  Rejected: { label: '已驳回', color: 'red' },
}

export const APPROVAL_MODE_MAP: Record<string, { label: string; color: string }> = {
  Any: { label: '抢单模式', color: 'arcoblue' },
  All: { label: '会签模式', color: 'purple' },
  Majority: { label: '投票模式', color: 'cyan' },
}

export const VARIABLE_TYPE_OPTIONS = [
  { label: 'String', value: 'String' },
  { label: 'Number', value: 'Number' },
  { label: 'Bool', value: 'Bool' },
  { label: 'JSON', value: 'Json' },
  { label: 'Secret', value: 'Secret' },
]
