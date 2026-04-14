import request from './request'
import type { LoginRequest, RegisterRequest, LoginResponse, RegisterResponse, ChangePasswordRequest, TenantOption, UserProfile } from '../types/auth'

export const authApi = {
  login: (data: LoginRequest) =>
    request.post<any, { data: LoginResponse }>('/auth/login', data),

  register: (data: RegisterRequest) =>
    request.post<any, { data: RegisterResponse }>('/auth/register', data),

  listTenants: () =>
    request.get<any, { data: TenantOption[] }>('/auth/tenants'),

  changePassword: (data: ChangePasswordRequest) =>
    request.post<any, { data: void }>('/auth/change-password', data),

  getProfile: () =>
    request.get<any, { data: UserProfile }>('/auth/profile'),
}
