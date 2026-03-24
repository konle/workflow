import request from './request'
import type { TaskInstanceEntity } from '../types/task'

export const taskInstanceApi = {
  create: (data: Partial<TaskInstanceEntity>) =>
    request.post<any, { data: TaskInstanceEntity }>('/task/instance', data),

  list: () =>
    request.get<any, { data: TaskInstanceEntity[] }>('/task/instance'),

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
