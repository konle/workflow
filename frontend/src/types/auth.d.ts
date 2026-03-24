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
