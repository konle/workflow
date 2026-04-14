import request from './request'
import type {
  ApiKeyListItem,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
} from '../types/apikey'

export const apiKeyApi = {
  list: () =>
    request.get<any, { data: ApiKeyListItem[] }>('/api-keys'),

  create: (data: CreateApiKeyRequest) =>
    request.post<any, { data: CreateApiKeyResponse }>('/api-keys', data),

  revoke: (id: string) =>
    request.delete<any, { data: void }>(`/api-keys/${id}`),
}
