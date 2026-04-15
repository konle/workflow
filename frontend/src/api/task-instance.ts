import request from './request'
import type { TaskInstanceEntity, ListTaskInstancesParams } from '../types/task'
import type { PaginatedData } from '../types/pagination'

export interface CreateTaskInstanceRequest {
  task_id: string
  context?: Record<string, unknown>
}

export const taskInstanceApi = {
  create: (data: CreateTaskInstanceRequest) =>
    request.post<any, { data: TaskInstanceEntity }>('/task/instance', data),

  list: (params?: ListTaskInstancesParams) =>
    request.get<any, { data: PaginatedData<TaskInstanceEntity> }>(
      '/task/instance',
      params ? { params } : {},
    ),

  get: (id: string) =>
    request.get<any, { data: TaskInstanceEntity }>(`/task/instance/${id}`),

  update: (id: string, data: Partial<TaskInstanceEntity>) =>
    request.put<any, { data: TaskInstanceEntity }>(`/task/instance/${id}`, data),

  execute: (id: string) =>
    request.post<any, { data: TaskInstanceEntity }>(`/task/instance/${id}/execute`),

  retry: (id: string) =>
    request.post<any, { data: TaskInstanceEntity }>(`/task/instance/${id}/retry`),

  cancel: (id: string) =>
    request.post<any, { data: TaskInstanceEntity }>(`/task/instance/${id}/cancel`),
}
