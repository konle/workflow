import type { JsonValue } from './common'

export type TaskType = 'Http' | 'IfCondition' | 'ContextRewrite' | 'Parallel' | 'ForkJoin' | 'SubWorkflow' | 'Grpc' | 'Approval'
export type TaskStatus = 'Draft' | 'Published'
export type TaskInstanceStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Canceled'
export type HttpMethod = 'Get' | 'Post' | 'Put' | 'Delete' | 'Head'
export type ParallelMode = 'Rolling' | 'Batch'
export type MergeMode = 'Merge' | 'Replace'

export type FormValueType = 'String' | 'Number' | 'Bool' | 'Json' | 'Variable'

export interface FormField {
  key: string
  value: string | number | boolean | JsonValue
  type: FormValueType
  description?: string
}

export interface TaskHttpTemplate {
  url: string
  method: HttpMethod
  headers: FormField[]
  body: FormField[]
  form: FormField[]
  retry_count: number
  retry_delay: number
  timeout: number
  success_condition: string | null
}

export interface IfConditionTemplate {
  condition: string
  name: string
  then_task: string | null
  else_task: string | null
}

export interface ContextRewriteTemplate {
  name: string
  script: string
  merge_mode: MergeMode
}

export interface ParallelTemplate {
  items_path: string
  item_alias: string
  task_template: TaskTemplate
  concurrency: number
  mode: ParallelMode
  max_failures: number | null
}

export interface ForkJoinTaskItem {
  task_key: string
  name: string
  task_template: TaskTemplate
}

export interface ForkJoinTemplate {
  tasks: ForkJoinTaskItem[]
  concurrency: number
  mode: ParallelMode
  max_failures: number | null
}

export interface SubWorkflowTemplate {
  workflow_meta_id: string
  workflow_version: number
  input_mapping: JsonValue | null
  output_path: string | null
  timeout: number | null
}

export type ApprovalMode = 'Any' | 'All' | 'Majority'

export interface ApproverRule {
  User?: string
  Role?: string
  ContextVariable?: string
}

export interface ApprovalTemplate {
  name: string
  title: string
  description: string | null
  approvers: ApproverRule[]
  approval_mode: ApprovalMode
  timeout: number | null
}

export type TaskTemplate =
  | { Http: TaskHttpTemplate }
  | { IfCondition: IfConditionTemplate }
  | { ContextRewrite: ContextRewriteTemplate }
  | { Parallel: ParallelTemplate }
  | { ForkJoin: ForkJoinTemplate }
  | { SubWorkflow: SubWorkflowTemplate }
  | { Approval: ApprovalTemplate }
  | 'Grpc'

export interface TaskEntity {
  id: string
  tenant_id: string
  name: string
  task_type: TaskType
  task_template: TaskTemplate
  description: string
  status: TaskStatus
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export interface WorkflowCallerContext {
  workflow_instance_id: string
  node_id: string
  parent_task_instance_id: string | null
  item_index: number | null
}

export interface TaskInstanceEntity {
  id: string
  tenant_id: string
  task_id: string
  task_name: string
  task_type: TaskType
  task_template: TaskTemplate
  task_status: TaskInstanceStatus
  task_instance_id: string
  created_at: string
  updated_at: string
  deleted_at: string | null
  input: JsonValue | null
  output: JsonValue | null
  error_message: string | null
  execution_duration: number | null
  caller_context: WorkflowCallerContext | null
}
