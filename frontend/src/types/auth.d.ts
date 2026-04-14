export interface LoginRequest {
  username: string
  password: string
  tenant_id: string
}

export interface RegisterRequest {
  username: string
  email: string
  password: string
}

export interface LoginResponse {
  token: string
  user_id: string
  username: string
}

export interface RegisterResponse {
  user_id: string
  username: string
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

export interface TenantOption {
  tenant_id: string
  name: string
}

export interface UserProfile {
  user_id: string
  username: string
  email: string
  is_super_admin: boolean
  status: string
  created_at: string
}
