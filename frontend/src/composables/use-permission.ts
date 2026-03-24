import { computed } from 'vue'
import { useAuthStore } from '../stores/auth'
import type { Permission, TenantRole } from '../types/user'

const ROLE_PERMISSIONS: Record<string, Permission[]> = {
  TenantAdmin: ['UserManage', 'TemplateWrite', 'InstanceExecute', 'ReadOnly'],
  Developer: ['TemplateWrite', 'InstanceExecute', 'ReadOnly'],
  Operator: ['InstanceExecute', 'ReadOnly'],
  Viewer: ['ReadOnly'],
}

export function usePermission() {
  const auth = useAuthStore()

  function hasPermission(perm: Permission): boolean {
    if (auth.isSuperAdmin) return true
    if (!auth.role) return false
    return (ROLE_PERMISSIONS[auth.role] || []).includes(perm)
  }

  const canWrite = computed(() => hasPermission('TemplateWrite'))
  const canExecute = computed(() => hasPermission('InstanceExecute'))
  const canManageUsers = computed(() => hasPermission('UserManage'))
  const isSuperAdmin = computed(() => auth.isSuperAdmin)

  return { hasPermission, canWrite, canExecute, canManageUsers, isSuperAdmin }
}
