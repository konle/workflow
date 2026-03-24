import request from './request'
import type { UserTenantRole, AssignRoleRequest } from '../types/user'

export const userApi = {
  list: () =>
    request.get<any, { data: UserTenantRole[] }>('/users'),

  assignRole: (data: AssignRoleRequest) =>
    request.post<any, { data: UserTenantRole }>('/users', data),

  updateRole: (userId: string, data: AssignRoleRequest) =>
    request.put<any, { data: UserTenantRole }>(`/users/${userId}`, data),

  removeRole: (userId: string) =>
    request.delete<any, { data: void }>(`/users/${userId}`),
}
