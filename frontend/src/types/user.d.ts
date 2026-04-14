export type TenantRole = 'TenantAdmin' | 'Developer' | 'Operator' | 'Viewer'
export type UserStatus = 'Active' | 'Disabled'
export type Permission = 'TenantManage' | 'UserManage' | 'TemplateWrite' | 'InstanceExecute' | 'ReadOnly'

export interface UserRoleInfo {
  user_id: string
  username: string
  email: string
  tenant_id: string
  role: TenantRole
  created_at: string
}

export interface UserTenantRole {
  user_id: string
  tenant_id: string
  role: TenantRole
  created_at: string
}

export interface AssignRoleRequest {
  username: string
  role: string
}

export interface CreateUserRequest {
  username: string
  email: string
  role: string
}

export interface CreateUserResponse {
  username: string
  initial_password: string
}
