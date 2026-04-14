export type ApiKeyStatus = 'Active' | 'Revoked'

export interface ApiKeyListItem {
  id: string
  name: string
  key_prefix: string
  role: string
  expires_at: string | null
  token_ttl_secs: number
  last_used_at: string | null
  status: ApiKeyStatus
  created_by: string
  created_at: string
}

export interface CreateApiKeyRequest {
  name: string
  role: string
  expires_at?: string | null
  token_ttl_secs?: number
}

export interface CreateApiKeyResponse {
  id: string
  name: string
  key: string
  key_prefix: string
  role: string
  expires_at: string | null
  token_ttl_secs: number
  created_at: string
}

export interface TokenExchangeRequest {
  key: string
}

export interface TokenExchangeResponse {
  access_token: string
  token_type: string
  expires_in: number
}
