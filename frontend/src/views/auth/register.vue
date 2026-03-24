<template>
  <div class="login-page">
    <div class="login-card">
      <h2 class="login-title">注册账号</h2>
      <a-form :model="form" @submit-success="handleRegister" layout="vertical">
        <a-form-item field="username" label="用户名" :rules="[{ required: true, message: '请输入用户名' }]">
          <a-input v-model="form.username" placeholder="请输入用户名" />
        </a-form-item>
        <a-form-item field="email" label="邮箱" :rules="[{ required: true, message: '请输入邮箱' }, { type: 'email', message: '邮箱格式不正确' }]">
          <a-input v-model="form.email" placeholder="请输入邮箱" />
        </a-form-item>
        <a-form-item field="password" label="密码" :rules="[{ required: true, message: '请输入密码' }]">
          <a-input-password v-model="form.password" placeholder="请输入密码" />
        </a-form-item>
        <a-form-item>
          <a-button type="primary" html-type="submit" :loading="loading" long>
            注册
          </a-button>
        </a-form-item>
      </a-form>
      <div class="login-footer">
        已有账号？<router-link to="/login">去登录</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { authApi } from '../../api/auth'
import { Notification } from '@arco-design/web-vue'

const router = useRouter()
const loading = ref(false)
const form = reactive({ username: '', email: '', password: '' })

async function handleRegister() {
  loading.value = true
  try {
    await authApi.register(form)
    Notification.success({ content: '注册成功，请登录' })
    router.push('/login')
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
