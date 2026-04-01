# 工作流引擎前端架构设计文档

本文档基于后端 API 路由与架构文档，设计配套的前端系统。涵盖技术选型、路由规划、页面拆解、状态管理、权限体系、可视化编排器等核心模块。

---

## 1. 技术选型

| 维度 | 选型 | 理由 |
|------|------|------|
| 框架 | **Vue 3** + Composition API | 响应式系统成熟，生态丰富，适合企业级中后台 |
| 语言 | **TypeScript** | 与后端 Rust 的强类型理念一致，提前捕获前端类型错误 |
| 构建 | **Vite** | 冷启动极快，原生 ESM，HMR 体验优于 Webpack |
| UI 组件库 | **Arco Design Vue** | 字节跳动出品，组件丰富，主题定制能力强，表格/表单/树形组件质量高 |
| 路由 | **Vue Router 4** | Vue 生态标配，支持路由守卫做权限拦截 |
| 状态管理 | **Pinia** | 官方推荐，TypeScript 友好，轻量直觉 |
| HTTP 客户端 | **Axios** | 拦截器机制成熟，统一处理 `code != 0` 的错误响应 |
| 工作流画布 | **Vue Flow** (基于 ReactFlow 的 Vue 移植) | 专业的 DAG 可视化编辑库，节点/边/布局开箱即用 |
| 代码编辑器 | **Monaco Editor** | 用于 Rhai 脚本（ContextRewrite / IfCondition）和 JSON 编辑 |
| 图标 | **@arco-design/web-vue/es/icon** | 与 UI 库统一风格 |
| 国际化 | **vue-i18n** | 预留多语言能力 |

---

## 2. 项目目录结构

```
frontend/
├── public/
├── src/
│   ├── api/                          # API 请求层
│   │   ├── request.ts                # Axios 实例 + 拦截器
│   │   ├── auth.ts
│   │   ├── tenant.ts
│   │   ├── user.ts
│   │   ├── task.ts
│   │   ├── task-instance.ts
│   │   ├── workflow.ts
│   │   ├── workflow-instance.ts
│   │   ├── variable.ts
│   │   └── approval.ts
│   │
│   ├── assets/                       # 静态资源
│   │
│   ├── components/                   # 全局通用组件
│   │   ├── layout/
│   │   │   ├── app-layout.vue        # 主布局骨架
│   │   │   ├── sidebar-menu.vue      # 侧边导航
│   │   │   ├── header-bar.vue        # 顶栏（租户切换 + 用户菜单）
│   │   │   └── breadcrumb.vue
│   │   ├── permission/
│   │   │   └── permission-guard.vue  # 权限指令/组件
│   │   └── common/
│   │       ├── status-tag.vue        # 状态标签（统一色彩映射）
│   │       ├── confirm-action.vue    # 确认操作弹窗
│   │       ├── json-viewer.vue       # JSON 只读展示
│   │       └── empty-state.vue       # 空状态占位
│   │
│   ├── composables/                  # 可复用逻辑 (Hooks)
│   │   ├── use-auth.ts              # 登录态 & Token
│   │   ├── use-permission.ts        # 权限判断
│   │   ├── use-tenant.ts            # 当前租户上下文
│   │   ├── use-pagination.ts        # 分页逻辑
│   │   └── use-polling.ts           # 实例状态轮询
│   │
│   ├── router/
│   │   ├── index.ts                 # 路由定义
│   │   └── guards.ts               # 全局前置守卫
│   │
│   ├── stores/                      # Pinia Stores
│   │   ├── auth.ts
│   │   ├── tenant.ts
│   │   └── app.ts                   # 全局 UI 状态（侧边栏折叠等）
│   │
│   ├── types/                       # TypeScript 类型定义
│   │   ├── api.d.ts                 # API 响应包装
│   │   ├── auth.d.ts
│   │   ├── tenant.d.ts
│   │   ├── user.d.ts
│   │   ├── task.d.ts
│   │   ├── workflow.d.ts
│   │   ├── variable.d.ts
│   │   └── approval.d.ts
│   │
│   ├── utils/
│   │   ├── token.ts                 # JWT 存取
│   │   ├── constants.ts             # 枚举/常量映射
│   │   └── format.ts               # 日期/状态格式化
│   │
│   ├── views/                       # 页面视图
│   │   ├── auth/
│   │   │   ├── login.vue
│   │   │   └── register.vue
│   │   │
│   │   ├── dashboard/
│   │   │   └── index.vue            # 仪表盘概览
│   │   │
│   │   ├── tenant/                  # SuperAdmin 专属
│   │   │   ├── list.vue
│   │   │   └── detail.vue
│   │   │
│   │   ├── user/                    # TenantAdmin+
│   │   │   └── list.vue
│   │   │
│   │   ├── task/
│   │   │   ├── template/
│   │   │   │   ├── list.vue         # 原子任务模板列表
│   │   │   │   └── editor.vue       # 原子任务模板编辑
│   │   │   └── instance/
│   │   │       ├── list.vue         # 任务实例列表
│   │   │       └── detail.vue       # 任务实例详情
│   │   │
│   │   ├── workflow/
│   │   │   ├── meta/
│   │   │   │   ├── list.vue         # 工作流 Meta 列表
│   │   │   │   └── detail.vue       # 工作流 Meta 详情（含版本管理）
│   │   │   ├── editor/              # 可视化工作流编排器
│   │   │   │   ├── index.vue        # 编排器主页面（Vue Flow + 属性面板）
│   │   │   │   ├── workflow-node.vue
│   │   │   │   ├── condition-node.vue
│   │   │   │   ├── published-task-ref-fields.vue   # 已发布原子任务：选择 + 任务信息 + 运行参数
│   │   │   │   ├── subworkflow-ref-fields.vue      # 子工作流：Meta / 版本 / 运行参数 / 超时
│   │   │   │   ├── parallel-inner-task-panel.vue   # Parallel 子任务类型与子配置入口
│   │   │   │   ├── workflow-editor-form-utils.ts   # 编排器表单字段 build / 序列化
│   │   │   │   └── parallel-inner-task-utils.ts    # Parallel 内层 task_template 默认 / hydrate / 保存
│   │   │   └── instance/
│   │   │       ├── list.vue         # 工作流实例列表
│   │   │       └── detail.vue       # 工作流实例详情（含节点状态）
│   │   │
│   │   ├── variable/
│   │   │   ├── tenant-list.vue      # 租户级变量管理
│   │   │   └── meta-list.vue        # 工作流模板级变量管理（嵌入 Meta 详情）
│   │   │
│   │   ├── approval/                # 审批中心
│   │   │   ├── list.vue             # 审批列表（我的待办 + 全部审批）
│   │   │   └── detail.vue           # 审批详情 & 决策操作
│   │   │
│   │   └── error/
│   │       ├── 403.vue
│   │       └── 404.vue
│   │
│   ├── App.vue
│   └── main.ts
│
├── index.html
├── vite.config.ts
├── tsconfig.json
└── package.json
```

---

## 3. 路由设计

路由结构与后端 API 路由保持语义对齐，同时适配前端 SPA 导航习惯。

### 3.1 路由表

| 路径 | 页面 | 权限要求 | 说明 |
|------|------|----------|------|
| `/login` | `auth/login.vue` | 公开 | 登录页 |
| `/register` | `auth/register.vue` | 公开 | 注册页 |
| `/` | `dashboard/index.vue` | 已登录 | 仪表盘 |
| `/tenants` | `tenant/list.vue` | SuperAdmin | 租户管理列表 |
| `/tenants/:id` | `tenant/detail.vue` | SuperAdmin | 租户详情 |
| `/users` | `user/list.vue` | UserManage | 用户管理 |
| `/tasks` | `task/template/list.vue` | ReadOnly+ | 原子任务模板列表 |
| `/tasks/create` | `task/template/editor.vue` | TemplateWrite | 创建任务模板 |
| `/tasks/:id/edit` | `task/template/editor.vue` | TemplateWrite | 编辑任务模板 |
| `/tasks/instances` | `task/instance/list.vue` | ReadOnly+ | 任务实例列表 |
| `/tasks/instances/:id` | `task/instance/detail.vue` | ReadOnly+ | 任务实例详情 |
| `/workflows` | `workflow/meta/list.vue` | ReadOnly+ | 工作流 Meta 列表 |
| `/workflows/create` | `workflow/meta/editor.vue` | TemplateWrite | 创建工作流（含表单定义） |
| `/workflows/:metaId` | `workflow/meta/detail.vue` | ReadOnly+ | 工作流 Meta 详情 & 版本管理 & 表单编辑 |
| `/workflows/:metaId/editor` | `workflow/editor/index.vue` | TemplateWrite | 可视化编排器（新建版本） |
| `/workflows/:metaId/editor/:version` | `workflow/editor/index.vue` | TemplateWrite | 可视化编排器（编辑已有版本） |
| `/workflows/instances` | `workflow/instance/list.vue` | ReadOnly+ | 工作流实例列表 |
| `/workflows/instances/:id` | `workflow/instance/detail.vue` | ReadOnly+ | 工作流实例详情 |
| `/variables` | `variable/tenant-list.vue` | ReadOnly+ | 租户级变量管理 |
| `/approvals` | `approval/list.vue` | 已登录 | 审批中心（我的待办 + 全部审批） |
| `/approvals/:id` | `approval/detail.vue` | 已登录 | 审批详情 & 决策 |

### 3.2 路由守卫策略

```
beforeEach(to, from):
  1. to 是公开路由 (/login, /register) → 放行
  2. Token 不存在或已过期 → 重定向 /login
  3. 解析 JWT payload，写入 authStore (user_id, tenant_id, role, is_super_admin)
  4. 匹配 to.meta.permission，调用 role.has_permission(perm) 判断
  5. 无权限 → 重定向 /403
```

---

## 4. API 请求层设计

### 4.1 Axios 拦截器 (request.ts)

```typescript
// 请求拦截：注入 Authorization header
instance.interceptors.request.use(config => {
  const token = getToken()
  if (token) config.headers.Authorization = `Bearer ${token}`
  return config
})

// 响应拦截：统一错误处理
instance.interceptors.response.use(
  response => {
    const { code, message, data } = response.data
    if (code !== 0) {
      Notification.error({ title: '请求失败', content: message })
      return Promise.reject(new Error(message))
    }
    // 剥离 code，只返回 { message, data }
    return { message, data }
  },
  error => {
    if (error.response?.status === 401) {
      // Token 失效，跳转登录
      clearToken()
      router.push('/login')
    }
    return Promise.reject(error)
  }
)
```

### 4.2 API 模块示例 (workflow.ts)

```typescript
export const workflowApi = {
  // Meta CRUD
  createMeta: (data: CreateWorkflowMetaRequest) =>
    request.post<WorkflowMeta>('/workflow/meta', data),
  listMeta: () =>
    request.get<WorkflowMeta[]>('/workflow/meta'),
  getMeta: (metaId: string) =>
    request.get<WorkflowMeta>(`/workflow/meta/${metaId}`),
  updateMeta: (metaId: string, data: UpdateWorkflowMetaRequest) =>
    request.put(`/workflow/meta/${metaId}`, data),
  deleteMeta: (metaId: string) =>
    request.delete(`/workflow/meta/${metaId}`),

  // Template (versioned)
  saveTemplate: (metaId: string, data: WorkflowEntity) =>
    request.post(`/workflow/meta/${metaId}/template`, data),
  getTemplate: (metaId: string, version: number) =>
    request.get<WorkflowEntity>(`/workflow/meta/${metaId}/template/${version}`),
  deleteTemplate: (metaId: string, version: number) =>
    request.delete(`/workflow/meta/${metaId}/template/${version}`),
  publishTemplate: (metaId: string, version: number) =>
    request.post(`/workflow/meta/${metaId}/template/${version}/publish`),

  // Instance
  createInstance: (data: CreateWorkflowInstanceReq) =>
    request.post<WorkflowInstance>('/workflow/instance', data),
  listInstances: () =>
    request.get<WorkflowInstance[]>('/workflow/instance'),
  getInstance: (id: string) =>
    request.get<WorkflowInstance>(`/workflow/instance/${id}`),
  executeInstance: (id: string) =>
    request.post<WorkflowInstance>(`/workflow/instance/${id}/execute`),
  cancelInstance: (id: string) =>
    request.post<WorkflowInstance>(`/workflow/instance/${id}/cancel`),
  retryInstance: (id: string) =>
    request.post<WorkflowInstance>(`/workflow/instance/${id}/retry`),
  resumeInstance: (id: string) =>
    request.post<WorkflowInstance>(`/workflow/instance/${id}/resume`),
}
```

---

## 5. TypeScript 类型映射

前端 TypeScript 类型与后端 Rust 实体一一对应。

### 5.1 枚举映射

```typescript
// 对应后端 shared/workflow.rs
type WorkflowStatus = 'Draft' | 'Published' | 'Deleted' | 'Archived'
type WorkflowInstanceStatus = 'Pending' | 'Running' | 'Await' | 'Completed' | 'Failed' | 'Canceled' | 'Suspended'
type TaskStatus = 'Draft' | 'Published'
type TaskInstanceStatus = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Canceled'
type TaskType = 'Http' | 'IfCondition' | 'ContextRewrite' | 'Parallel' | 'ForkJoin' | 'SubWorkflow' | 'Grpc' | 'Approval'
type NodeExecutionStatus = 'Pending' | 'Running' | 'Success' | 'Failed' | 'Suspended' | 'Skipped'

type TenantRole = 'TenantAdmin' | 'Developer' | 'Operator' | 'Viewer'
type TenantStatus = 'Active' | 'Suspended' | 'Deleted'
type VariableScope = 'Tenant' | 'WorkflowMeta'
type VariableType = 'String' | 'Number' | 'Bool' | 'Json' | 'Secret'

// 对应后端 shared/form.rs
type FormValueType = 'String' | 'Number' | 'Bool' | 'Json' | 'Variable'
```

### 5.2 实体类型

```typescript
// 表单字段 (对应后端 shared/form.rs)
// FormValueType 决定引擎如何处理 value：
//   String / Number / Bool / Json → 字面量，原样使用
//   Variable → 模板字符串，引擎渲染 {{}} 占位符
interface FormField {
  key: string
  value: string | number | boolean | JsonValue
  type: FormValueType
  description?: string
}

// HTTP 任务模板
interface TaskHttpTemplate {
  url: string                       // 支持 {{变量}} 模板，约定始终做模板渲染
  method: HttpMethod
  headers: FormField[]              // 请求头列表，Variable 类型支持 {{变量}}
  body: FormField[]                 // 请求体字段列表，每条构成 body JSON 的一个 key
  form: FormField[]                 // 用户输入表单定义，运行时渲染到 URL/Headers/Body
  retry_count: number
  retry_delay: number
  timeout: number
  success_condition: string | null
}

// WorkflowMeta
interface WorkflowMeta {
  workflow_meta_id: string
  tenant_id: string
  name: string
  description: string
  status: WorkflowStatus
  form: FormField[]
  created_at: string
  updated_at: string
}

// 创建/更新 WorkflowMeta 请求（与后端 DTO 对齐，不含 id/tenant_id/时间戳）
interface CreateWorkflowMetaRequest {
  name: string
  description: string
  status: WorkflowStatus
  form?: FormField[]
}
interface UpdateWorkflowMetaRequest {
  name: string
  description: string
  status: WorkflowStatus
  form?: FormField[]
}

// WorkflowEntity（版本化模板，即 DAG 定义）
interface WorkflowEntity {
  workflow_meta_id: string
  version: number
  status: WorkflowStatus
  entry_node: string
  nodes: WorkflowNodeEntity[]
  created_at: string
  updated_at: string
}

interface WorkflowNodeEntity {
  node_id: string
  node_type: TaskType
  task_id?: string | null  // 引用的原子任务模板ID（Http/Approval/gRPC 节点使用）
  config: TaskTemplate     // 联合类型，按 node_type 区分；原子任务节点存储选中任务的 template 快照
  context: JsonValue
  next_node: string | null
}

// TaskTemplate 联合类型（tagged union，serde 默认按 key 区分）
type TaskTemplate =
  | { Http: TaskHttpTemplate }
  | { IfCondition: IfConditionTemplate }
  | { ContextRewrite: ContextRewriteTemplate }
  | { Parallel: ParallelTemplate }
  | { ForkJoin: ForkJoinTemplate }
  | { SubWorkflow: SubWorkflowTemplate }
  | { Approval: ApprovalTemplate }

// 审批模板
type ApprovalMode = 'Any' | 'All' | 'Majority'
type ApproverRule = { User: string } | { Role: string } | { ContextVariable: string }

interface ApprovalTemplate {
  name: string
  title: string
  description: string | null
  approvers: ApproverRule[]
  approval_mode: ApprovalMode
  timeout: number | null
}

// 审批实例
type ApprovalStatus = 'Pending' | 'Approved' | 'Rejected'
type Decision = 'Approve' | 'Reject'

interface ApprovalInstanceEntity {
  id: string
  tenant_id: string
  workflow_instance_id: string
  node_id: string
  title: string
  description: string | null
  approval_mode: ApprovalMode
  approvers: string[]
  decisions: ApprovalDecision[]
  status: ApprovalStatus
  created_at: string
  updated_at: string
  expires_at: string | null
}

interface ApprovalDecision {
  user_id: string
  decision: Decision
  comment: string | null
  decided_at: string
}

// TaskInstanceEntity
interface TaskInstance {
  id: string
  tenant_id: string
  task_id: string
  task_name: string
  task_type: TaskType
  task_template: TaskTemplate
  task_status: TaskInstanceStatus
  task_instance_id: string
  input: JsonValue | null
  output: JsonValue | null
  error_message: string | null
  execution_duration: number | null
  created_at: string
  updated_at: string
}
```

---

## 6. 页面详细设计

### 6.1 登录页 (`/login`)

| 区域 | 说明 |
|------|------|
| 表单 | 用户名、密码、租户选择（下拉或手动输入 tenant_id） |
| 提交 | `POST /api/v1/auth/login` → 存 Token → 跳转 `/` |
| 注册入口 | 链接跳转 `/register` |

SuperAdmin 登录后，`tenant_id` 字段为默认管理租户；进入系统后可通过顶栏租户切换器切换到任意租户上下文。

### 6.2 仪表盘 (`/`)

全局概览页，按当前租户维度展示：

| 卡片 | 数据来源 | 说明 |
|------|----------|------|
| 任务模板总数 | `GET /task` → count | 原子任务模板数量 |
| 工作流模板总数 | `GET /workflow/meta` → count | 工作流 Meta 数量 |
| 运行中实例数 | `GET /workflow/instance` → filter Running | 当前活跃实例 |
| 最近失败实例 | `GET /workflow/instance` → filter Failed → top 5 | 快速定位问题 |

> **备注**：仪表盘的聚合统计理想情况下应由后端提供专用的统计 API（如 `GET /api/v1/stats`），避免前端拉全量列表后在本地计数。当前后端暂无此接口，V1 版本可先用列表接口的 length 做近似展示，后续迭代补充。

---

### 6.3 租户管理 (`/tenants`) — SuperAdmin 专属

**列表页**

| 列 | 字段 | 说明 |
|----|------|------|
| 租户名称 | `name` | |
| 状态 | `status` | Active / Suspended / Deleted，用色彩标签区分 |
| 配额 | `max_workflows` / `max_instances` | |
| 创建时间 | `created_at` | |
| 操作 | — | 编辑 / 暂停 / 删除 |

**详情/编辑** — 抽屉 (Drawer) 形式。

---

### 6.4 用户管理 (`/users`) — TenantAdmin+

**列表页**

| 列 | 字段 | 说明 |
|----|------|------|
| 用户ID | `user_id` | |
| 角色 | `role` | TenantAdmin / Developer / Operator / Viewer |
| 加入时间 | `created_at` | |
| 操作 | — | 修改角色 / 移除 |

**邀请用户** — 弹窗表单：输入 `user_id` + 选择 `role`，调用 `POST /users`。

---

### 6.5 原子任务模板 (`/tasks`)

#### 6.5.1 列表页

| 列 | 字段 | 说明 |
|----|------|------|
| 任务名称 | `name` | 可点击进入编辑 |
| 类型 | `task_type` | 用图标+文字区分 Http / Grpc / Approval 等 |
| 状态 | `status` | Draft / Published |
| 描述 | `description` | 溢出截断 |
| 更新时间 | `updated_at` | |
| 操作 | — | 编辑 / 删除 / 创建实例 |

#### 6.5.2 编辑页 (`/tasks/create`, `/tasks/:id/edit`)

页面布局：上方为基础信息表单，下方为**任务类型专属配置区**。

**基础信息**

| 字段 | 控件 |
|------|------|
| name | Input |
| description | Textarea |
| task_type | Select（Http / Grpc / Approval） |
| status | Radio（Draft / Published） |

**任务类型配置区** — 根据 `task_type` 动态渲染不同的子表单组件：

| TaskType | 子表单组件 | 核心字段 |
|----------|------------|----------|
| Http | `task-http-form.vue` | url, method, headers (FormField[] KV编辑器+常用预设), body (FormField[] 字段编辑器), form (FormField[] 用户表单定义), retry_count, retry_delay, timeout, success_condition |
| Grpc | `task-grpc-form.vue` | (预留) |
| Approval | `task-approval-form.vue` | title, description, approval_mode (Any/All/Majority), approvers (动态规则列表: User/Role/ContextVariable), timeout |

**HTTP 任务编辑器分为三个区块**：

1. **请求配置**（任务设计者填写）：URL、Method、Headers（FormField[] 动态 KV 编辑器 + 常用 Header 快捷标签）、Body（FormField[] 字段编辑器，每行 key+value+type+description）
2. **用户表单定义**（定义最终用户看到的输入表单）：Form（FormField[] 编辑器），此处定义的字段在工作流实例创建时展示给用户填写，值通过 `{{key}}` 在 URL/Headers/Body 中引用
3. **运行参数**：timeout、retry_count、retry_delay、success_condition

**FormField.type（FormValueType）语义**：
- `String` / `Number` / `Bool` / `Json`：字面量，原样使用，不做模板渲染
- `Variable`：模板字符串，引擎执行时渲染 `{{}}` 占位符

> 注意：`IfCondition`、`ContextRewrite`、`Parallel`、`ForkJoin`、`SubWorkflow` 是编排层节点类型，不作为独立原子任务模板暴露在此页面。它们只在**工作流可视化编排器**中作为节点类型使用。

---

### 6.6 任务实例 (`/tasks/instances`)

#### 6.6.1 列表页

| 列 | 字段 | 说明 |
|----|------|------|
| 实例ID | `task_instance_id` | 可点击进入详情 |
| 任务名称 | `task_name` | |
| 类型 | `task_type` | |
| 状态 | `task_status` | Pending / Running / Completed / Failed / Canceled |
| 耗时 | `execution_duration` | 毫秒 → 可读格式 |
| 创建时间 | `created_at` | |
| 操作 | — | 执行 / 重试 / 取消（按状态机规则启禁用） |

**操作按钮的状态机约束**：
- 执行：仅 `Pending` 状态可用
- 重试：仅 `Failed` 状态可用
- 取消：仅 `Pending` / `Failed` 状态可用

#### 6.6.2 详情页 (`/tasks/instances/:id`)

| 区域 | 内容 |
|------|------|
| 基础信息卡片 | 实例ID、任务名、类型、状态、耗时、创建/更新时间 |
| Input 区域 | JSON Viewer 展示 `input`（渲染后的实际请求） |
| Output 区域 | JSON Viewer 展示 `output`（执行结果） |
| Error 区域 | 仅在 `error_message` 非空时展示，红色告警卡片 |

---

### 6.7 工作流 Meta (`/workflows`)

#### 6.7.1 列表页

| 列 | 字段 | 说明 |
|----|------|------|
| 工作流名称 | `name` | 可点击进入详情 |
| 状态 | `status` | Draft / Published |
| 表单字段数 | `form.length` | 快速了解该工作流需要多少输入 |
| 更新时间 | `updated_at` | |
| 操作 | — | 查看详情 / 新建版本 / 删除 |

"创建工作流"按钮跳转至独立的创建页面 `/workflows/create`（`workflow/meta/editor.vue`）。

#### 6.7.2 创建页面 (`/workflows/create`)

独立的全页面编辑器，包含：
- 基础字段：name、description、status
- **工作流表单定义**：与任务模板 `editor.vue` 共享相同的 form-list 布局，每行包含 key、默认值（根据 type 渲染对应输入控件）、type（String/Number/Bool/Json）、description、删除按钮
- 表单定义在所有版本间共享，属于 `WorkflowMetaEntity.form`

#### 6.7.3 详情页 (`/workflows/:metaId`)

页面分为三个 Tab：

**Tab 1: 基础信息**
- 编辑 Meta 基础字段（name, description, status）
- **工作流表单定义**：与创建页面相同的 form-list 编辑器，可查看和编辑 `form[]` 字段
- 保存时仅提交 `{ name, description, status, form }` 字段（对应后端 `UpdateWorkflowMetaRequest`）

**Tab 2: 版本管理**
- 列出该 Meta 下所有版本（`WorkflowEntity`）
- 每个版本卡片展示：version 号、节点数量、状态、创建时间
- 操作：查看/编辑（跳转编排器）、删除、基于此版本创建实例
- "发起实例"弹窗根据 `meta.form` 动态渲染用户输入表单

**Tab 3: 模板变量**
- 嵌入 `variable/meta-list.vue`，管理该 Meta 的工作流模板级变量
- 对应 API：`GET/POST/PUT/DELETE /workflow/meta/{meta_id}/variables`

---

### 6.8 可视化工作流编排器 (`/workflows/:metaId/editor`)

这是系统最核心、最复杂的页面，用于以拖拽方式构建工作流的 DAG 图。

#### 6.8.1 页面布局

```
┌─────────────────────────────────────────────────────────┐
│ 顶栏: [← 返回] 工作流名称 v{version}    [保存] [发布]  │
├────────┬────────────────────────────────┬───────────────┤
│        │                                │               │
│ 节点   │                                │  属性面板     │
│ 面板   │       画布区域                  │  (右侧)      │
│ (左侧) │       (Vue Flow)               │               │
│        │                                │               │
│ ─Http  │    [Start] → [Http] → [If]    │  选中节点的   │
│ ─If    │                  ↓     ↓       │  配置表单     │
│ ─Rewrite│            [Parallel] [End]   │               │
│ ─Parallel│                              │               │
│ ─ForkJoin│                              │               │
│ ─SubWF │                                │               │
│        │                                │               │
├────────┴────────────────────────────────┴───────────────┤
│ 底栏: 缩放控制 | 小地图 | 节点数统计                     │
└─────────────────────────────────────────────────────────┘
```

#### 6.8.2 节点面板 (左侧)

可拖拽的节点类型列表，每种类型有独立图标和色彩标识：

| 节点类型 | 图标色 | 描述 | 属于原子任务? |
|----------|--------|------|:---:|
| Http | 蓝色 | HTTP 请求 | ✅ |
| Grpc | 紫色 | gRPC 调用 | ✅ |
| Approval | 橙色 | 人工审批 | ✅ |
| IfCondition | 黄色 | 条件分支 | ❌ (控制流) |
| ContextRewrite | 青色 | 上下文重写 | ❌ (控制流) |
| Parallel | 绿色 | 同构并发容器 | ❌ (容器) |
| ForkJoin | 绿色深 | 异构并发容器 | ❌ (容器) |
| SubWorkflow | 灰色 | 子工作流 | ❌ (编排) |

从面板拖拽节点到画布，自动创建 `WorkflowNodeEntity`，生成唯一 `node_id`。

#### 6.8.3 画布区域 (中央)

基于 **Vue Flow** 实现，使用自定义节点组件：

**自定义节点组件**：

| 组件文件 | Vue Flow type | 用途 | Handle 配置 |
|---------|---------------|------|-------------|
| `workflow-node.vue` | `workflow` | 所有普通节点 | 顶部 1 个 target + 底部 1 个 source |
| `condition-node.vue` | `condition` | IfCondition 节点 | 顶部 1 个 target + 底部 2 个 source（`then` 绿色 / `else` 红色） |

**属性面板复用子组件**（与 `index.vue` 同目录）：

| 文件 | 职责 |
|------|------|
| `published-task-ref-fields.vue` | Http / Approval / Grpc 节点：已发布任务下拉、只读任务信息、运行参数编辑 |
| `subworkflow-ref-fields.vue` | SubWorkflow 节点：工作流 Meta、版本、信息、运行参数、超时 |
| `parallel-inner-task-panel.vue` | Parallel 节点：**子任务类型**（HTTP / gRPC / 子工作流）及对应配置区 |
| `workflow-editor-form-utils.ts` | `EditorFormField`、模板 form 与编辑器行的双向转换 |
| `parallel-inner-task-utils.ts` | 内层 `TaskTemplate` 默认值、从 API 回填编辑器状态、保存时写入 `config.Parallel.task_template` |

**连线交互**：

- 从节点底部 source handle 拖拽到另一节点顶部 target handle 创建边（`@connect` 事件处理）
- 普通节点：一个 source handle，同一节点只允许一条出边（新连线自动替换旧连线）
- IfCondition 节点：两个 source handle（`then`/`else`），各自独立连线
- 点击边可在右侧属性面板查看信息或删除，也支持 Delete/Backspace 键删除选中边

**边的样式**：

| 边类型 | 颜色 | 标签 | 来源 |
|--------|------|------|------|
| 普通 `next_node` | 默认灰 | 无 | source handle（默认） |
| IfCondition Then | 绿色 `#00B42A` | `True` | sourceHandle = `then` |
| IfCondition Else | 红色 `#F53F3F` | `False` | sourceHandle = `else` |

**其他画布功能**：

- 拖拽平移、滚轮缩放、框选多节点
- Delete/Backspace 删除选中的边
- 容器节点 (Parallel / ForkJoin) 内部子任务配置在属性面板中完成

**节点展示名称**：

- 引用原子任务的节点（Http / Approval / gRPC）：选中任务后展示**任务名称**
- **SubWorkflow**：展示**工作流名称 + 版本**
- **Parallel**：在已配置内层子任务时，展示为 `node_id · 子任务名` 或 `node_id · 子工作流名 v{version}`；否则回退 `node_id (Parallel)`
- **IfCondition** / **ContextRewrite**：展示配置中的 `name`（`config.name`）
- 以上均不可用时，回退为 `node_id (类型)` 形式

**删除节点**：

- 选中节点后按 **Delete** / **Backspace**，或点击工具栏 **「删除节点」**
- 删除节点时**同时移除**与该节点相连的全部边，**不做**自动桥接（不会在上下游之间自动补边）
- 操作前弹出**确认对话框**

#### 6.8.4 属性面板 (右侧)

点击画布中的节点，右侧面板动态渲染该节点类型对应的配置表单。

| 节点类型 | 交互模式 | 属性面板内容 |
|----------|---------|-------------|
| **Http** | **引用任务** | 由 `published-task-ref-fields.vue` 渲染：1. 任务选择（搜索下拉，按 task_type=Http 过滤已发布任务）→ 2. 任务信息（只读：URL、Method、超时、重试）→ 3. 运行参数（根据 form 定义渲染，type 仅可选原始类型或 Variable） |
| **Approval** | **引用任务** | 同上组件，按 task_type=Approval 过滤，信息展示：标题、审批模式，运行参数同 form |
| **gRPC** | **引用任务** | 同上组件，按 task_type=Grpc 过滤 |
| **SubWorkflow** | **引用工作流** | 由 `subworkflow-ref-fields.vue` 渲染：1. 工作流选择（搜索下拉）→ 版本选择 → 2. 工作流信息（只读：名称、状态）→ 3. 运行参数（来自 WorkflowMeta.form，type 仅可选原始类型或 Variable）→ 超时 |
| **IfCondition** | **内联配置** | 条件名称、Rhai 表达式、分支连接（Then/Else 目标节点由画布连线决定，属性面板只读显示） |
| **ContextRewrite** | **内联配置** | 名称、Rhai 脚本、合并模式 (Merge / Replace) |
| **Parallel** | **内联 + 子任务引用** | 1. **数据与并发**：`items_path`、`item_alias`、`concurrency`、`mode` (Rolling / Batch)、`max_failures`。2. **子任务**（`parallel-inner-task-panel.vue`）：先选 **子任务类型** — **HTTP** / **gRPC** / **子工作流**（与架构文档约定一致：内层须为具备 TaskExecutor 的原子能力；子工作流用于嵌套编排）。HTTP/gRPC 路径复用与顶层原子节点相同的「已发布任务 + 任务信息 + 运行参数」交互（内嵌 `published-task-ref-fields.vue`）；子工作流路径复用「Meta + 版本 + 运行参数 + 超时」（内嵌 `subworkflow-ref-fields.vue`）。持久化字段为 `config.Parallel.task_template`（`TaskTemplate` 联合类型之一），节点级 `task_id` 仍为 `null`。编辑器在 `node.data` 上维护 `parallelInnerKind`、`parallelInnerTaskId`、`parallelInnerFormFields`、`parallelSubWorkflow*` 等仅用于 UI 与保存组装的辅助状态；加载模板后会对「子工作流 + 已选 Meta」补拉版本列表（`GET .../workflow/meta/{id}/template` 列表）。 |
| **ForkJoin** | **内联配置** | 子任务列表（JSON）、concurrency、mode、max_failures |
| **通用字段** | — | node_id (只读)、context (JSON 编辑器) |

> **运行参数 type 约束**：在编排器中填写 form 参数时，每个字段的 type 选择器仅提供两个选项：该字段的原始类型（由任务/工作流设计者定义）和 Variable（引用上下文变量）。这确保类型安全的同时允许动态取值。

> **点击边**：选中一条边后，右侧属性面板显示连线信息（源节点 → 目标节点、分支类型），并提供"删除连线"按钮。

#### 6.8.5 保存校验

保存时执行悬空节点检测（严格模式）：

1. 遍历所有节点，统计每个节点是否至少有一条入边或出边
2. 起始节点（仅有出边无入边）视为合法
3. 若存在完全孤立的悬空节点（既无入边也无出边）：
   - **阻止保存**
   - 弹出警告通知，列出悬空节点 ID
   - 将悬空节点边框标红并闪烁动画，方便用户定位
4. 用户连线或删除悬空节点后，再次保存即可通过校验

5. **配置完整性检测**（在通过悬空节点校验之后继续检查）：
   - **Http / Approval / gRPC**：必须已选择 `task_id`
   - **SubWorkflow**：必须填写 `meta_id` 与 `version`
   - **IfCondition**：必须配置条件表达式，且 **Then / Else** 两路均已连线
   - **ContextRewrite**：必须填写脚本
   - **Parallel**：必须填写非空 `items_path`；且须完整配置**内层子任务**：HTTP 须选择已发布模板（或等价地具备非空 URL 的模板快照）；gRPC 须选择已发布模板；子工作流须选择 Meta 与版本
   - **ForkJoin**：子任务列表须为合法 **JSON** 且通过校验
   - 未满足上述条件的节点：**橙色边框 + 脉冲动画**，保存被阻止并提示

6. **回环检测**：对 DAG 做 **DFS** 环检测。若存在有向环，**阻止保存**并展示错误信息（含环上路径），便于用户修正。

> 设计理由：工作流模板保存后要执行，悬空节点没有意义，只会造成数据冗余和执行引擎歧义。编辑器处于 Draft 阶段，用户随时可调整，不需要"先存一半"。

#### 6.8.6 数据模型转换

**画布 → API**：保存时将 Vue Flow 的 nodes/edges 转换为后端 `WorkflowEntity` 格式：

```
Vue Flow nodes[] + edges[]
    ↓ 按 sourceHandle 分类边
    - 无 sourceHandle / 默认 → nextNodeMap (source → target)
    - sourceHandle = "then"  → thenMap (source → target)
    - sourceHandle = "else"  → elseMap (source → target)
    ↓ 构建节点
    - 普通节点: next_node = nextNodeMap[node_id]
    - IfCondition: next_node = null, config.IfCondition.then_task = thenMap[node_id], else_task = elseMap[node_id]
    ↓ 转换
WorkflowEntity {
  workflow_meta_id,
  version,
  status,
  entry_node,   // 入口节点ID（优先取入度为0的节点，兜底为第一个节点）
  nodes: [
    {
      node_id: "node_1",
      node_type: "Http",
      task_id: "5c4a8a2c-...",
      config: { Http: { url: "...", method: "Get", ... } },
      context: {},
      next_node: "node_2"
    },
    {
      node_id: "node_3",
      node_type: "IfCondition",
      config: { IfCondition: { name: "...", condition: "...", then_task: "node_4", else_task: "node_5" } },
      context: {},
      next_node: null
    },
    {
      node_id: "node_6",
      node_type: "Parallel",
      task_id: null,
      config: {
        Parallel: {
          items_path: "users",
          item_alias: "user",
          task_template: { Http: { url: "...", method: "Get", ... } },
          concurrency: 10,
          mode: "Rolling",
          max_failures: 2
        }
      },
      context: {},
      next_node: "node_7"
    },
    ...
  ]
}
    ↓ 提交
POST /api/v1/workflow/meta/{metaId}/template
```

**API → 画布**：加载时将 `WorkflowEntity.nodes[]` 转换为 Vue Flow 的 nodes/edges，利用自动布局算法（dagre）进行初始排列。

#### 6.8.6 IfCondition 节点的边处理

`IfCondition` 是唯一拥有分支出边的节点：
- 模板中 `then_task` / `else_task` 存储目标 `node_id`
- 画布中渲染为两个输出锚点（标注 "True" / "False"）
- `next_node` 字段在 If 节点上保持为 `null`，分支目标由 `IfConditionTemplate.then_task` / `else_task` 承载

#### 6.8.7 Undo / Redo

- **Ctrl+Z**：撤销；**Ctrl+Shift+Z**：重做（标准重做快捷键）
- 基于**快照**：每次对 `nodes` + `edges` 做**深拷贝**入栈
- 历史栈**最多保留 50 步**
- 在以下操作后会推入新快照：添加节点、连线、删除节点/边、选择任务或子工作流、切换子工作流版本、**Parallel 子任务类型/模板/子工作流或内层运行参数变更**等
- 画布**左下角**展示快捷键提示文案

#### 6.8.8 版本生命周期（编排器与 Meta 详情联动）

- **新建版本**：前端自动计算下一版本号 = 当前 Meta 下已有版本的 **max(version) + 1**
- **保存**：与后端 upsert 对齐；同一 **Draft** 版本可反复保存覆盖，**Published / Archived** 版本不可通过保存更新（后端 422）
- **已发布版本**：打开编排器时为**只读模式**（输入禁用、禁止拖拽与连线变更、**不显示保存按钮**等）
- **版本列表**（Meta 详情 Tab）：每条版本展示状态标签——**Draft** 灰色、**Published** 绿色、**Archived** 橙色；**Draft** 显示 **「发布」**；非 Draft 时入口文案为 **「查看」** 而非 **「编辑」**

---

### 6.9 工作流实例 (`/workflows/instances`)

#### 6.9.1 列表页

| 列 | 字段 | 说明 |
|----|------|------|
| 实例ID | `workflow_instance_id` | 可点击进入详情 |
| 工作流 Meta | `workflow_meta_id` | 关联的 Meta 名称 |
| 版本 | `workflow_version` | |
| 状态 | `status` | 带色彩标签 |
| 当前节点 | `current_node` | 显示正在执行的节点 |
| 创建时间 | `created_at` | |
| 操作 | — | 执行 / 取消 / 重试 / 恢复 |

**操作按钮状态机约束** (对应后端 `WorkflowInstanceStatus::can_transition_to`)：

| 操作 | 可用状态 |
|------|----------|
| 执行 (execute) | Pending |
| 取消 (cancel) | Failed, Suspended |
| 重试 (retry) | Failed |
| 恢复 (resume) | Suspended |

#### 6.9.2 详情页 (`/workflows/instances/:id`)

这是一个**只读的工作流执行视图**，复用编排器的画布组件，但处于只读模式。

**页面布局**：与编排器类似的三栏布局，但左侧面板替换为"执行信息"面板。

| 区域 | 内容 |
|------|------|
| 左侧面板 | 实例基础信息：状态、版本、创建时间、上下文 (JSON Viewer) |
| 中央画布 | 只读 DAG 图，每个节点按 `NodeExecutionStatus` 实时着色 |
| 右侧面板 | 点击节点后展示：节点状态、`task_instance.input`（解析后入参）、`task_instance.output`（结果）、Error；**不再**使用已移除的 `node.output` |

**节点着色规则**：

| NodeExecutionStatus | 节点颜色 | 边框样式 |
|----|------|------|
| Pending | 灰色 | 虚线 |
| Running | 蓝色 + 呼吸动画 | 实线 |
| Success | 绿色 | 实线 |
| Failed | 红色 | 实线 |
| Suspended | 橙色 | 虚线 |
| Skipped | 灰色淡化 | 点线 |

**自动轮询**：当实例处于 `Running` / `Await` / `Suspended` 等非终态时，前端以 5 秒间隔轮询 `GET /workflow/instance/{id}`，实时刷新节点状态。到达终态后停止轮询。

---

### 6.10 创建工作流实例 (弹窗)

从工作流 Meta 详情页的版本卡片，或列表页的"发起实例"按钮触发。

弹窗内容：
1. 显示所选 `workflow_meta_id` 和 `version`
2. **动态表单**：根据 `WorkflowMetaEntity.form[]` 动态渲染输入控件

| FormField.type (FormValueType) | 渲染控件 |
|-------------------------------|----------|
| `String` | Input |
| `Number` | InputNumber |
| `Bool` | Switch |
| `Json` | Monaco Editor (JSON mode) |
| `Variable` | Input（提示支持 `{{变量}}` 模板语法） |

3. 提交时将表单数据组装为 `context` JSON，调用 `POST /api/v1/workflow/instance`
4. 成功后可选择"立即执行"（追加调用 `POST /api/v1/workflow/instance/{id}/execute`）或"仅创建"

---

### 6.11 变量管理 (`/variables`)

#### 租户级变量

| 列 | 字段 | 说明 |
|----|------|------|
| 变量名 | `key` | |
| 类型 | `variable_type` | String / Number / Bool / Json / Secret |
| 值 | `value` | Secret 类型显示为 `******` |
| 描述 | `description` | |
| 创建者 | `created_by` | |
| 操作 | — | 编辑 / 删除 |

Secret 类型变量的特殊处理：
- 列表中 `value` 列显示掩码
- 编辑时 `value` 输入框为 password 类型
- 创建/编辑 Secret 变量的操作按钮仅对 TenantAdmin+ 角色可见

---

### 6.12 审批中心 (`/approvals`)

#### 6.12.1 列表页

双 Tab 布局：

**Tab 1: 我的待办** — `GET /api/v1/approvals`（当前用户待审批列表）
**Tab 2: 全部审批** — `GET /api/v1/approvals/all`（仅 TenantAdmin+ 可见）

| 列 | 字段 | 说明 |
|----|------|------|
| 审批标题 | `title` | 可点击进入详情 |
| 审批模式 | `approval_mode` | Any=抢单 / All=会签 / Majority=投票 |
| 状态 | `status` | Pending(橙色) / Approved(绿色) / Rejected(红色) |
| 进度 | `decisions.length / approvers.length` | 如 "1/3" |
| 关联工作流 | `workflow_instance_id` | 可跳转到工作流实例详情 |
| 创建时间 | `created_at` | |
| 操作 | — | "去审批"按钮（仅 Pending 且在 approvers 中且未决策） |

#### 6.12.2 详情页 (`/approvals/:id`)

| 区域 | 内容 |
|------|------|
| 基础信息卡片 | 标题、描述、审批模式、状态、关联工作流实例、创建时间、过期时间 |
| 审批人列表 | 所有审批人，每人旁标注已通过/已拒绝/待审批 |
| 决策历史 | 时间线展示每个人的决策（决策结果、评论、时间） |
| 操作区 | 通过/拒绝按钮 + 评论输入框（仅 Pending + 当前用户是审批人且未决策时显示） |

#### 6.12.3 审批状态色彩

| 审批状态 | 色彩 | Arco Tag |
|----------|------|----------|
| Pending | 橙色 | `orange` |
| Approved | 绿色 | `green` |
| Rejected | 红色 | `red` |

#### 6.12.4 审批模式标签

| 模式 | 标签 | 色彩 |
|------|------|------|
| Any | 抢单模式 | `arcoblue` |
| All | 会签模式 | `purple` |
| Majority | 投票模式 | `cyan` |

---

## 7. 权限体系前端实现

### 7.1 角色信息来源

JWT Token 中包含 `role` 和 `is_super_admin`，登录后解析并存入 `authStore`：

```typescript
interface AuthState {
  token: string
  userId: string
  username: string
  tenantId: string
  role: TenantRole | 'SuperAdmin'
  isSuperAdmin: boolean
}
```

### 7.2 权限判断 composable

```typescript
// use-permission.ts
export function usePermission() {
  const auth = useAuthStore()

  const hasPermission = (perm: Permission): boolean => {
    if (auth.isSuperAdmin) return true
    return roleHasPermission(auth.role, perm)
  }

  const canWrite = computed(() => hasPermission('TemplateWrite'))
  const canExecute = computed(() => hasPermission('InstanceExecute'))
  const canManageUsers = computed(() => hasPermission('UserManage'))
  const isSuperAdmin = computed(() => auth.isSuperAdmin)

  return { hasPermission, canWrite, canExecute, canManageUsers, isSuperAdmin }
}
```

### 7.3 UI 层权限控制

| 层级 | 实现方式 |
|------|----------|
| **路由层** | `router.beforeEach` 守卫，检查 `to.meta.permission`，无权限重定向 403 |
| **菜单层** | `sidebar-menu.vue` 根据权限过滤菜单项（Viewer 看不到"用户管理"、"租户管理"） |
| **按钮层** | `v-if="canWrite"` 控制"创建"/"编辑"/"删除"按钮的显隐 |
| **表单层** | 只读角色进入编辑页时表单禁用 (`disabled`)，保存按钮隐藏 |

### 7.4 侧边导航菜单结构

```
📊 仪表盘                    ← 所有角色
📋 原子任务
  └─ 任务模板                ← ReadOnly+
  └─ 任务实例                ← ReadOnly+
🔀 工作流
  └─ 工作流管理              ← ReadOnly+
  └─ 工作流实例              ← ReadOnly+
🔐 变量管理                  ← ReadOnly+
📝 审批中心                  ← 已登录（所有角色）
👥 用户管理                  ← UserManage (TenantAdmin+)
🏢 租户管理                  ← SuperAdmin only
```

---

## 8. SuperAdmin 租户切换机制

SuperAdmin 登录后，顶栏右侧显示**租户切换器** (Select 组件)：

1. 调用 `GET /api/v1/tenants` 拉取租户列表
2. 选中租户后更新 `authStore.tenantId`
3. 后续所有 API 请求自动携带新的 `tenant_id`（通过请求拦截器注入 `X-Tenant-Id` Header 或重新签发 Token）
4. 切换后自动刷新当前页面数据

普通用户不展示此组件，其 `tenantId` 在登录时即固定。

---

## 9. 全局状态管理 (Pinia Stores)

| Store | 职责 | 持久化 |
|-------|------|--------|
| `authStore` | Token、用户信息、角色、当前 tenant_id | localStorage |
| `tenantStore` | 租户列表缓存（SuperAdmin 用） | 否 |
| `appStore` | 侧边栏折叠状态、主题模式等 UI 全局状态 | localStorage |

> 业务数据（任务列表、工作流列表等）**不放入全局 Store**，而是在各页面组件内通过 `composable` + `ref` 管理，避免全局状态膨胀。仅当跨页面共享（如 authStore）时才使用 Pinia。

---

## 10. 状态标签色彩规范

为保证全系统状态展示的视觉一致性，统一定义色彩映射：

### 10.1 工作流实例状态

| 状态 | 色彩 | Arco Tag 类型 |
|------|------|--------------|
| Pending | 灰色 | `default` |
| Running | 蓝色 | `arcoblue` |
| Await | 橙黄色 | `orangered` |
| Completed | 绿色 | `green` |
| Failed | 红色 | `red` |
| Canceled | 灰色 | `default` |
| Suspended | 橙色 | `orange` |

### 10.2 任务实例状态

沿用上表中相同状态名的色彩。

### 10.3 模板状态

| 状态 | 色彩 |
|------|------|
| Draft | 灰色 |
| Published | 绿色 |

---

## 11. 错误处理策略

| 场景 | 处理方式 |
|------|----------|
| API 返回 `code != 0` | 拦截器统一弹出 `Notification.error`，显示 `message` |
| HTTP 401 | 清除 Token，重定向 `/login` |
| HTTP 403 | 弹出"无权限"提示，不跳转（保留当前页面） |
| HTTP 404 | 跳转 `/404` |
| HTTP 5xx | 弹出"服务器错误"通知 |
| 网络断开 | 弹出"网络异常"通知，提供"重试"按钮 |

---

## 12. 后续迭代方向

| 优先级 | 方向 | 说明 |
|:------:|------|------|
| P0 | 仪表盘统计 API | 后端补充 `GET /api/v1/stats`，避免前端全量拉取计数 |
| P0 | 分页与搜索 | 所有列表接口补充分页参数 (`page`, `page_size`) 和关键字搜索 |
| P1 | WebSocket 实时推送 | 替代轮询，实例状态变更由后端主动推送 |
| P1 | 工作流执行日志流 | 实时查看节点级执行日志 |
| P2 | 工作流模板导入/导出 | JSON 格式的模板序列化，支持跨租户迁移 |
| P2 | 暗色主题 | Arco Design 原生支持，需适配画布节点色彩 |

---

## 13. API 路由完整参照表

以下为前端需要对接的全部后端 API 端点，统一前缀 `/api/v1`：

```
# ──── 公开接口 ────
POST   /auth/login
POST   /auth/register

# ──── SuperAdmin 专属 ────
POST   /tenants
GET    /tenants
GET    /tenants/{id}
PUT    /tenants/{id}
DELETE /tenants/{id}
POST   /tenants/{id}/suspend

# ──── 用户管理 (TenantAdmin+) ────
POST   /users
GET    /users
PUT    /users/{user_id}
DELETE /users/{user_id}

# ──── 以下接口均自动携带 tenant_id 隔离 ────

# 租户变量
POST   /variables
GET    /variables
GET    /variables/{id}
PUT    /variables/{id}
DELETE /variables/{id}

# 审批
GET    /approvals                          # 我的待审批列表
GET    /approvals/all                      # 全部审批 (TenantAdmin+)
GET    /approvals/{id}                     # 审批详情
POST   /approvals/{id}/decide              # 提交审批决策

# 原子任务模板
POST   /task
GET    /task
GET    /task/{id}
PUT    /task/{id}
DELETE /task/{id}

# 任务实例
POST   /task/instance
GET    /task/instance
GET    /task/instance/{id}
PUT    /task/instance/{id}
POST   /task/instance/{id}/execute
POST   /task/instance/{id}/retry
POST   /task/instance/{id}/cancel

# 工作流 Meta
POST   /workflow/meta
GET    /workflow/meta
GET    /workflow/meta/{workflow_meta_id}
PUT    /workflow/meta/{workflow_meta_id}
DELETE /workflow/meta/{workflow_meta_id}

# 工作流版本模板
POST   /workflow/meta/{workflow_meta_id}/template
GET    /workflow/meta/{workflow_meta_id}/template/{version}
DELETE /workflow/meta/{workflow_meta_id}/template/{version}
POST   /workflow/meta/{workflow_meta_id}/template/{version}/publish

# 工作流模板变量
POST   /workflow/meta/{meta_id}/variables
GET    /workflow/meta/{meta_id}/variables
GET    /workflow/meta/{meta_id}/variables/{var_id}
PUT    /workflow/meta/{meta_id}/variables/{var_id}
DELETE /workflow/meta/{meta_id}/variables/{var_id}

# 工作流实例
POST   /workflow/instance
GET    /workflow/instance
GET    /workflow/instance/{id}
POST   /workflow/instance/{id}/execute
POST   /workflow/instance/{id}/cancel
POST   /workflow/instance/{id}/retry
POST   /workflow/instance/{id}/resume
```
