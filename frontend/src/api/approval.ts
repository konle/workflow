import request from './request'
import type { ApprovalInstanceEntity, Decision } from '../types/approval'

export const approvalApi = {
  listMine: () =>
    request.get<any, { data: ApprovalInstanceEntity[] }>('/approvals'),

  listAll: () =>
    request.get<any, { data: ApprovalInstanceEntity[] }>('/approvals/all'),

  get: (id: string) =>
    request.get<any, { data: ApprovalInstanceEntity }>(`/approvals/${id}`),

  decide: (id: string, data: { decision: Decision; comment?: string }) =>
    request.post<any, { data: ApprovalInstanceEntity }>(`/approvals/${id}/decide`, data),
}
