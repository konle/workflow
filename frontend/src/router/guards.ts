import type { Router } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import type { Permission, TenantRole } from '../types/user'

const ROLE_PERMISSIONS: Record<string, Permission[]> = {
  TenantAdmin: ['UserManage', 'TemplateWrite', 'InstanceExecute', 'ReadOnly'],
  Developer: ['TemplateWrite', 'InstanceExecute', 'ReadOnly'],
  Operator: ['InstanceExecute', 'ReadOnly'],
  Viewer: ['ReadOnly'],
}

function hasPermission(role: TenantRole | '', isSuperAdmin: boolean, perm: Permission): boolean {
  if (isSuperAdmin) return true
  if (!role) return false
  return (ROLE_PERMISSIONS[role] || []).includes(perm)
}

export function setupGuards(router: Router) {
  router.beforeEach((to, _from, next) => {
    if (to.meta.public) return next()

    const auth = useAuthStore()
    if (!auth.isLoggedIn) return next('/login')

    const requiredPerm = to.meta.permission as Permission | undefined
    if (requiredPerm && !hasPermission(auth.role as TenantRole, auth.isSuperAdmin, requiredPerm)) {
      return next('/403')
    }

    next()
  })
}
