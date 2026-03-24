export type VariableScope = 'Tenant' | 'WorkflowMeta'
export type VariableType = 'String' | 'Number' | 'Bool' | 'Json' | 'Secret'

export interface VariableEntity {
  id: string
  tenant_id: string
  scope: VariableScope
  scope_id: string
  key: string
  value: string
  variable_type: VariableType
  description: string | null
  created_by: string
  created_at: string
  updated_at: string
}

export interface CreateVariableRequest {
  key: string
  value: string
  variable_type: VariableType
  description?: string
}

export interface UpdateVariableRequest {
  value: string
  variable_type: VariableType
  description?: string
}
