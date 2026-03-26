import request from './request'
import type { TaskEntity } from '../types/task'

export const taskApi = {
  create: (data: Partial<TaskEntity>) =>
    request.post<any, { data: TaskEntity }>('/task', data),

  list: (params?: { task_type?: string }) =>
    request.get<any, { data: TaskEntity[] }>('/task', { params }),

  get: (id: string) =>
    request.get<any, { data: TaskEntity }>(`/task/${id}`),

  update: (id: string, data: Partial<TaskEntity>) =>
    request.put<any, { data: TaskEntity }>(`/task/${id}`, data),

  delete: (id: string) =>
    request.delete<any, { data: void }>(`/task/${id}`),
}
