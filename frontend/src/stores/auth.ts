import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getToken, setToken, clearToken } from '../utils/token'
import type { TenantRole } from '../types/user'

interface JwtPayload {
  sub: string
  username: string
  is_super_admin: boolean
  tenant_id: string
  role: string
  exp: number
}

function parseJwt(token: string): JwtPayload | null {
  try {
    const base64 = token.split('.')[1]
    return JSON.parse(atob(base64))
  } catch {
    return null
  }
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref(getToken() || '')
  const userId = ref('')
  const username = ref('')
  const tenantId = ref('')
  const role = ref<TenantRole | ''>('')
  const isSuperAdmin = ref(false)

  const isLoggedIn = computed(() => !!token.value)

  function init() {
    if (token.value) {
      const payload = parseJwt(token.value)
      if (payload && payload.exp * 1000 > Date.now()) {
        userId.value = payload.sub
        username.value = payload.username
        tenantId.value = payload.tenant_id
        role.value = payload.is_super_admin ? '' : (payload.role as TenantRole)
        isSuperAdmin.value = payload.is_super_admin
      } else {
        logout()
      }
    }
  }

  function login(newToken: string, user: { user_id: string; username: string }) {
    token.value = newToken
    setToken(newToken)
    const payload = parseJwt(newToken)
    if (payload) {
      userId.value = user.user_id
      username.value = user.username
      tenantId.value = payload.tenant_id
      role.value = payload.is_super_admin ? '' : (payload.role as TenantRole)
      isSuperAdmin.value = payload.is_super_admin
    }
  }

  function logout() {
    token.value = ''
    userId.value = ''
    username.value = ''
    tenantId.value = ''
    role.value = ''
    isSuperAdmin.value = false
    clearToken()
  }

  function switchTenant(newTenantId: string) {
    tenantId.value = newTenantId
  }

  init()

  return {
    token, userId, username, tenantId, role, isSuperAdmin,
    isLoggedIn, login, logout, switchTenant,
  }
})
