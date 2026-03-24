<template>
  <div>
    <a-page-header title="租户详情" @back="$router.push('/tenants')" />
    <a-card :loading="loading">
      <a-descriptions :data="descData" :column="2" bordered />
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { tenantApi } from '../../api/tenant'
import { formatDate } from '../../utils/format'
import type { TenantEntity } from '../../types/tenant'

const route = useRoute()
const loading = ref(false)
const tenant = ref<TenantEntity | null>(null)

const descData = computed(() => {
  if (!tenant.value) return []
  const t = tenant.value
  return [
    { label: '租户ID', value: t.tenant_id },
    { label: '名称', value: t.name },
    { label: '状态', value: t.status },
    { label: '描述', value: t.description },
    { label: '最大工作流', value: t.max_workflows ?? '无限制' },
    { label: '最大实例', value: t.max_instances ?? '无限制' },
    { label: '创建时间', value: formatDate(t.created_at) },
    { label: '更新时间', value: formatDate(t.updated_at) },
  ]
})

onMounted(async () => {
  loading.value = true
  try {
    const res = await tenantApi.get(route.params.id as string)
    tenant.value = res.data
  } catch {} finally { loading.value = false }
})
</script>
