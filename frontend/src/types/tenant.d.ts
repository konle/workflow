export type TenantStatus = 'Active' | 'Suspended' | 'Deleted'

export interface TenantEntity {
  tenant_id: string
  name: string
  description: string
  status: TenantStatus
  max_workflows: number | null
  max_instances: number | null
  created_at: string
  updated_at: string
}

export interface CreateTenantRequest {
  name: string
  description: string
}
