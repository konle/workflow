import request from './request'
import type { VariableEntity, CreateVariableRequest, UpdateVariableRequest } from '../types/variable'

export const variableApi = {
  // Tenant variables
  listTenant: () =>
    request.get<any, { data: VariableEntity[] }>('/variables'),

  createTenant: (data: CreateVariableRequest) =>
    request.post<any, { data: VariableEntity }>('/variables', data),

  getTenant: (id: string) =>
    request.get<any, { data: VariableEntity }>(`/variables/${id}`),

  updateTenant: (id: string, data: UpdateVariableRequest) =>
    request.put<any, { data: VariableEntity }>(`/variables/${id}`, data),

  deleteTenant: (id: string) =>
    request.delete<any, { data: void }>(`/variables/${id}`),

  // Workflow meta variables
  listMeta: (metaId: string) =>
    request.get<any, { data: VariableEntity[] }>(`/workflow/meta/${metaId}/variables`),

  createMeta: (metaId: string, data: CreateVariableRequest) =>
    request.post<any, { data: VariableEntity }>(`/workflow/meta/${metaId}/variables`, data),

  getMeta: (metaId: string, varId: string) =>
    request.get<any, { data: VariableEntity }>(`/workflow/meta/${metaId}/variables/${varId}`),

  updateMeta: (metaId: string, varId: string, data: UpdateVariableRequest) =>
    request.put<any, { data: VariableEntity }>(`/workflow/meta/${metaId}/variables/${varId}`, data),

  deleteMeta: (metaId: string, varId: string) =>
    request.delete<any, { data: void }>(`/workflow/meta/${metaId}/variables/${varId}`),
}
