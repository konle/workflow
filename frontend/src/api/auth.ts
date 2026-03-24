import request from './request'
import type { LoginRequest, RegisterRequest, LoginResponse, RegisterResponse } from '../types/auth'

export const authApi = {
  login: (data: LoginRequest) =>
    request.post<any, { data: LoginResponse }>('/auth/login', data),

  register: (data: RegisterRequest) =>
    request.post<any, { data: RegisterResponse }>('/auth/register', data),
}
