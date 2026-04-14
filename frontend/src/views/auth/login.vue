<template>
  <div class="login-page">
    <div class="login-card">
      <h2 class="login-title">Workflow Engine</h2>
      <a-form :model="form" @submit-success="handleLogin" layout="vertical">
        <a-form-item field="username" label="用户名" :rules="[{ required: true, message: '请输入用户名' }]">
          <a-input v-model="form.username" placeholder="请输入用户名" />
        </a-form-item>
        <a-form-item field="password" label="密码" :rules="[{ required: true, message: '请输入密码' }]">
          <a-input-password v-model="form.password" placeholder="请输入密码" />
        </a-form-item>
        <a-form-item field="tenant_id" label="租户" :rules="[{ required: true, message: '请选择租户' }]">
          <a-select
            v-model="form.tenant_id"
            placeholder="请选择租户"
            :loading="tenantsLoading"
          >
            <a-option
              v-for="t in tenants"
              :key="t.tenant_id"
              :value="t.tenant_id"
              :label="t.name"
            />
          </a-select>
        </a-form-item>
        <a-form-item>
          <a-button type="primary" html-type="submit" :loading="loading" long>
            登录
          </a-button>
        </a-form-item>
      </a-form>
      <div class="login-footer">
        还没有账号？<router-link to="/register">立即注册</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../../stores/auth'
import { authApi } from '../../api/auth'
import { Notification } from '@arco-design/web-vue'
import type { TenantOption } from '../../types/auth'

const router = useRouter()
const auth = useAuthStore()
const loading = ref(false)
const tenantsLoading = ref(false)
const tenants = ref<TenantOption[]>([])
const form = reactive({ username: '', password: '', tenant_id: '' })

onMounted(async () => {
  tenantsLoading.value = true
  try {
    const res = await authApi.listTenants()
    tenants.value = res.data
    if (tenants.value.length === 1) {
      form.tenant_id = tenants.value[0].tenant_id
    }
  } catch { /* ignore */ }
  finally { tenantsLoading.value = false }
})

async function handleLogin() {
  loading.value = true
  try {
    const res = await authApi.login(form)
    auth.login(res.data.token, { user_id: res.data.user_id, username: res.data.username })
    Notification.success({ content: '登录成功' })
    router.push('/')
  } catch { /* handled by interceptor */ }
  finally { loading.value = false }
}
</script>

<style scoped>
.login-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-fill-2);
}
.login-card {
  width: 400px;
  padding: 40px;
  background: var(--color-bg-2);
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}
.login-title {
  text-align: center;
  margin-bottom: 32px;
  font-size: 24px;
  font-weight: 600;
}
.login-footer {
  text-align: center;
  margin-top: 16px;
  color: var(--color-text-3);
}
</style>
