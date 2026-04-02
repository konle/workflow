import request from './request'
import type {
  WorkflowMetaEntity, WorkflowEntity, WorkflowInstanceEntity,
  CreateWorkflowMetaRequest, UpdateWorkflowMetaRequest, CreateWorkflowInstanceRequest,
} from '../types/workflow'

export const workflowApi = {
  createMeta: (data: CreateWorkflowMetaRequest) =>
    request.post<any, { data: WorkflowMetaEntity }>('/workflow/meta', data),

  listMeta: () =>
    request.get<any, { data: WorkflowMetaEntity[] }>('/workflow/meta'),

  getMeta: (metaId: string) =>
    request.get<any, { data: WorkflowMetaEntity }>(`/workflow/meta/${metaId}`),

  updateMeta: (metaId: string, data: UpdateWorkflowMetaRequest) =>
    request.put<any, { data: void }>(`/workflow/meta/${metaId}`, data),

  deleteMeta: (metaId: string) =>
    request.delete<any, { data: void }>(`/workflow/meta/${metaId}`),

  saveTemplate: (metaId: string, data: WorkflowEntity) =>
    request.post<any, { data: void }>(`/workflow/meta/${metaId}/template`, data),

  listTemplates: (metaId: string) =>
    request.get<any, { data: WorkflowEntity[] }>(`/workflow/meta/${metaId}/template`),

  getTemplate: (metaId: string, version: number) =>
    request.get<any, { data: WorkflowEntity }>(`/workflow/meta/${metaId}/template/${version}`),

  deleteTemplate: (metaId: string, version: number) =>
    request.delete<any, { data: void }>(`/workflow/meta/${metaId}/template/${version}`),

  publishTemplate: (metaId: string, version: number) =>
    request.post<any, { data: void }>(`/workflow/meta/${metaId}/template/${version}/publish`),

  archiveTemplate: (metaId: string, version: number) =>
    request.post<any, { data: void }>(`/workflow/meta/${metaId}/template/${version}/archive`),

  copyTemplate: (metaId: string, version: number) =>
    request.post<any, { data: void }>(`/workflow/meta/${metaId}/template/${version}/copy`),

  createInstance: (data: CreateWorkflowInstanceRequest) =>
    request.post<any, { data: WorkflowInstanceEntity }>('/workflow/instance', data),

  listInstances: () =>
    request.get<any, { data: WorkflowInstanceEntity[] }>('/workflow/instance'),

  getInstance: (id: string) =>
    request.get<any, { data: WorkflowInstanceEntity }>(`/workflow/instance/${id}`),

  executeInstance: (id: string) =>
    request.post<any, { data: WorkflowInstanceEntity }>(`/workflow/instance/${id}/execute`),

  cancelInstance: (id: string) =>
    request.post<any, { data: WorkflowInstanceEntity }>(`/workflow/instance/${id}/cancel`),

  retryInstance: (id: string) =>
    request.post<any, { data: WorkflowInstanceEntity }>(`/workflow/instance/${id}/retry`),

  resumeInstance: (id: string) =>
    request.post<any, { data: WorkflowInstanceEntity }>(`/workflow/instance/${id}/resume`),
}
