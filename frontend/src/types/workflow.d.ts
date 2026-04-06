import type { JsonValue } from './common'
import type { TaskType, TaskTemplate, TaskInstanceEntity } from './task'

export type WorkflowStatus = 'Draft' | 'Published' | 'Deleted' | 'Archived'
export type WorkflowInstanceStatus = 'Pending' | 'Running' | 'Await' | 'Completed' | 'Failed' | 'Canceled' | 'Suspended'
export type NodeExecutionStatus = 'Pending' | 'Running' | 'Success' | 'Failed' | 'Suspended' | 'Skipped'

export interface FormField {
  key: string
  value: string | number | boolean | JsonValue
  type: string
  description?: string
}

export interface WorkflowMetaEntity {
  workflow_meta_id: string
  tenant_id: string
  name: string
  description: string
  status: WorkflowStatus
  form: FormField[]
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export interface WorkflowNodeEntity {
  node_id: string
  node_type: TaskType
  task_id?: string | null
  config: TaskTemplate
  context: JsonValue
  next_node: string | null
}

export interface WorkflowEntity {
  workflow_meta_id: string
  version: number
  status: WorkflowStatus
  entry_node: string
  nodes: WorkflowNodeEntity[]
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export interface WorkflowNodeInstanceEntity {
  node_id: string
  node_type: TaskType
  task_instance: TaskInstanceEntity
  context: JsonValue
  next_node: string | null
  status: NodeExecutionStatus
  error_message: string | null
  created_at: string
  updated_at: string
}

export interface WorkflowCallerContext {
  workflow_instance_id: string
  node_id: string
  parent_task_instance_id: string | null
  item_index: number | null
}

export interface WorkflowInstanceEntity {
  workflow_instance_id: string
  tenant_id: string
  workflow_meta_id: string
  workflow_version: number
  status: WorkflowInstanceStatus
  created_at: string
  updated_at: string
  deleted_at: string | null
  context: JsonValue
  entry_node: string
  current_node: string
  nodes: WorkflowNodeInstanceEntity[]
  epoch: number
  locked_by: string | null
  locked_duration: number | null
  locked_at: string | null
  parent_context: WorkflowCallerContext | null
  depth: number
}

export interface CreateWorkflowMetaRequest {
  name: string
  description: string
  status: WorkflowStatus
  form?: FormField[]
}

export interface UpdateWorkflowMetaRequest {
  name: string
  description: string
  status: WorkflowStatus
  form?: FormField[]
}

export interface CreateWorkflowInstanceRequest {
  workflow_meta_id: string
  version: number
  context: JsonValue
}

/** GET /workflow/instance 查询参数，与后端 ListWorkflowInstancesRequest 对齐 */
export interface ListWorkflowInstancesParams {
  page?: number
  page_size?: number
  workflow_meta_id?: string
  version?: number
  status?: WorkflowInstanceStatus
  sort_by?: string
  sort_order?: string
}
