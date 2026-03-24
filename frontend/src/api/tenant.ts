import request from './request'
import type { TenantEntity, CreateTenantRequest } from '../types/tenant'

export const tenantApi = {
  create: (data: CreateTenantRequest) =>
    request.post<any, { data: TenantEntity }>('/tenants', data),

  list: () =>
    request.get<any, { data: TenantEntity[] }>('/tenants'),

  get: (id: string) =>
    request.get<any, { data: TenantEntity }>(`/tenants/${id}`),

  update: (id: string, data: CreateTenantRequest) =>
    request.put<any, { data: void }>(`/tenants/${id}`, data),

  delete: (id: string) =>
    request.delete<any, { data: void }>(`/tenants/${id}`),

  suspend: (id: string) =>
    request.post<any, { data: void }>(`/tenants/${id}/suspend`),
}
