<template>
  <div>
    <div style="display: flex; justify-content: flex-end; margin-bottom: 12px">
      <a-button v-if="canWrite" type="primary" size="small" @click="openCreate">添加变量</a-button>
    </div>
    <a-table :data="list" :columns="columns" :loading="loading" row-key="id" size="small">
      <template #variable_type="{ record }">
        <a-tag size="small">{{ record.variable_type }}</a-tag>
      </template>
      <template #value="{ record }">
        {{ record.variable_type === 'Secret' ? '******' : record.value }}
      </template>
      <template #action="{ record }">
        <a-space>
          <a-button v-if="canWrite" type="text" size="mini" @click="openEdit(record)">编辑</a-button>
          <a-popconfirm v-if="canWrite" content="确定删除？" @ok="handleDelete(record.id)">
            <a-button type="text" size="mini" status="danger">删除</a-button>
          </a-popconfirm>
        </a-space>
      </template>
    </a-table>

    <a-modal v-model:visible="showModal" :title="editTarget ? '编辑变量' : '添加变量'" @ok="handleSave" :ok-loading="saving">
      <a-form :model="form" layout="vertical">
        <a-form-item v-if="!editTarget" label="变量名" required>
          <a-input v-model="form.key" />
        </a-form-item>
        <a-form-item label="类型" required>
          <a-select v-model="form.variable_type">
            <a-option v-for="opt in VARIABLE_TYPE_OPTIONS" :key="opt.value" :value="opt.value" :label="opt.label" />
          </a-select>
        </a-form-item>
        <a-form-item label="值" required>
          <a-input-password v-if="form.variable_type === 'Secret'" v-model="form.value" />
          <a-textarea v-else-if="form.variable_type === 'Json'" v-model="form.value" :auto-size="{ minRows: 3 }" />
          <a-input v-else v-model="form.value" />
        </a-form-item>
        <a-form-item label="描述">
          <a-input v-model="form.description" />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue'
import { variableApi } from '../../api/variable'
import { usePermission } from '../../composables/use-permission'
import { VARIABLE_TYPE_OPTIONS } from '../../utils/constants'
import { Notification } from '@arco-design/web-vue'
import type { VariableEntity, VariableType } from '../../types/variable'

const props = defineProps<{ metaId: string }>()
const { canWrite } = usePermission()
const list = ref<VariableEntity[]>([])
const loading = ref(false)
const showModal = ref(false)
const saving = ref(false)
const editTarget = ref<VariableEntity | null>(null)
const form = reactive({ key: '', value: '', variable_type: 'String' as VariableType, description: '' })

const columns = [
  { title: '变量名', dataIndex: 'key' },
  { title: '类型', slotName: 'variable_type', width: 90 },
  { title: '值', slotName: 'value', ellipsis: true },
  { title: '描述', dataIndex: 'description', ellipsis: true },
  { title: '操作', slotName: 'action', width: 120 },
]

async function fetchList() {
  loading.value = true
  try {
    const res = await variableApi.listMeta(props.metaId)
    list.value = res.data
  } catch {} finally { loading.value = false }
}

function openCreate() {
  editTarget.value = null
  form.key = ''; form.value = ''; form.variable_type = 'String'; form.description = ''
  showModal.value = true
}

function openEdit(record: VariableEntity) {
  editTarget.value = record
  form.key = record.key
  form.value = record.variable_type === 'Secret' ? '' : record.value
  form.variable_type = record.variable_type
  form.description = record.description || ''
  showModal.value = true
}

async function handleSave() {
  saving.value = true
  try {
    if (editTarget.value) {
      await variableApi.updateMeta(props.metaId, editTarget.value.id, { value: form.value, variable_type: form.variable_type, description: form.description || undefined })
    } else {
      await variableApi.createMeta(props.metaId, { key: form.key, value: form.value, variable_type: form.variable_type, description: form.description || undefined })
    }
    Notification.success({ content: '保存成功' })
    showModal.value = false
    fetchList()
  } catch {} finally { saving.value = false }
}

async function handleDelete(id: string) {
  await variableApi.deleteMeta(props.metaId, id)
  Notification.success({ content: '已删除' })
  fetchList()
}

watch(() => props.metaId, fetchList)
onMounted(fetchList)
</script>
