export type TenantRole = 'TenantAdmin' | 'Developer' | 'Operator' | 'Viewer'
export type UserStatus = 'Active' | 'Disabled'
export type Permission = 'TenantManage' | 'UserManage' | 'TemplateWrite' | 'InstanceExecute' | 'ReadOnly'

export interface UserTenantRole {
  user_id: string
  tenant_id: string
  role: TenantRole
  created_at: string
}

export interface AssignRoleRequest {
  user_id: string
  role: string
}
