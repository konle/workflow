import request from './request'
import type { UserRoleInfo, AssignRoleRequest, CreateUserRequest, CreateUserResponse } from '../types/user'

export const userApi = {
  list: () =>
    request.get<any, { data: UserRoleInfo[] }>('/users'),

  assignRole: (data: AssignRoleRequest) =>
    request.post<any, { data: any }>('/users', data),

  updateRole: (username: string, data: AssignRoleRequest) =>
    request.put<any, { data: any }>(`/users/${username}`, data),

  removeRole: (username: string) =>
    request.delete<any, { data: void }>(`/users/${username}`),

  createUser: (data: CreateUserRequest) =>
    request.post<any, { data: CreateUserResponse }>('/users/create', data),
}
