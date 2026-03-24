<template>
  <div class="header-bar">
    <div class="header-left">
      <a-breadcrumb>
        <a-breadcrumb-item>{{ currentRouteName }}</a-breadcrumb-item>
      </a-breadcrumb>
    </div>
    <div class="header-right">
      <a-select
        v-if="isSuperAdmin"
        v-model="currentTenant"
        placeholder="选择租户"
        style="width: 200px; margin-right: 16px"
        @change="onTenantChange"
      >
        <a-option
          v-for="t in tenants"
          :key="t.tenant_id"
          :value="t.tenant_id"
          :label="t.name"
        />
      </a-select>
      <a-dropdown>
        <a-button type="text">
          <template #icon><icon-user /></template>
          {{ auth.username }}
        </a-button>
        <template #content>
          <a-doption @click="handleLogout">退出登录</a-doption>
        </template>
      </a-dropdown>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../../stores/auth'
import { usePermission } from '../../composables/use-permission'
import { tenantApi } from '../../api/tenant'
import { IconUser } from '@arco-design/web-vue/es/icon'
import type { TenantEntity } from '../../types/tenant'

const router = useRouter()
const route = useRoute()
const auth = useAuthStore()
const { isSuperAdmin } = usePermission()

const tenants = ref<TenantEntity[]>([])
const currentTenant = ref(auth.tenantId)

const currentRouteName = computed(() => {
  return (route.meta?.title as string) || route.name?.toString() || ''
})

onMounted(async () => {
  if (isSuperAdmin.value) {
    try {
      const res = await tenantApi.list()
      tenants.value = res.data
    } catch { /* ignore */ }
  }
})

function onTenantChange(val: string | number | boolean | Record<string, any> | (string | number | boolean | Record<string, any>)[]) {
  auth.switchTenant(val as string)
  router.go(0)
}

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}
.header-right {
  display: flex;
  align-items: center;
}
</style>
