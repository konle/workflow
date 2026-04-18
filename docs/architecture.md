# 分布式工作流引擎架构设计文档

本文档详细描述了本工作流引擎的架构设计、插件化系统、以及调度器与执行器之间的交互流转模型。

## 1. 核心架构设计

引擎采用了 **编排器 (Orchestrator)** 与 **执行器 (Executor)** 分离的微服务架构。通过分布式的消息队列（当前为 Apalis + Redis），将系统拆分为两类独立运行的 Worker 节点。

### 1.1 Worker 角色划分

1. **Workflow Worker (编排器)**
   * **职责**：负责工作流有向图的拓扑迭代、状态机流转、执行上下文（Context）的维护与传递、插件控制流的决策。
   * **行为**：它本身**不执行**任何诸如 HTTP 请求或脚本解析等重量级耗时操作。当执行到一个动作节点（如 HTTP 请求）时，它只负责生成一张任务执行工单，丢给队列，然后就挂起休眠，释放计算资源。

2. **Task Worker (执行器)**
   * **职责**：负责干脏活累活。从任务队列中抓取任务，调用具体的 `TaskExecutor`（如 `HttpTaskExecutor`）发起实际的网络请求、执行脚本等。
   * **行为**：执行完成后，负责将结果（Output、Error等）打包成事件（Event），反向投递给工作流队列，通知对应的 Workflow Worker 醒来收工。

### 1.2 架构交互图

```mermaid
sequenceDiagram
    participant WW as Workflow Worker (Engine)
    participant RQ as Redis Queue
    participant TW as Task Worker (Executor)
    participant DB as MongoDB

    Note over WW, DB: 1. 节点执行阶段 (Scatter)
    WW->>DB: 1. 读取 WorkflowInstance
    WW->>WW: 2. Manager 调用 Plugin::execute (如 HttpPlugin)
    WW->>RQ: 3. 发送 ExecuteTaskJob (包含 CallerContext)
    WW->>DB: 4. 将 WorkflowInstance 置为 Await 并保存状态（让出 CPU）

    Note over RQ, DB: 2. 任务消费与执行阶段
    RQ->>TW: 5. 消费 ExecuteTaskJob
    TW->>DB: 6. 获取 TaskInstanceEntity (包含请求参数)
    TW->>TW: 7. TaskManager 路由到对应的 TaskExecutor 并执行网络请求
    TW->>DB: 8. 更新 TaskInstanceEntity (落盘 Input/Output)
    
    Note over TW, WW: 3. 结果回调阶段 (Gather & Signal)
    TW->>RQ: 9. 投递 ExecuteWorkflowJob (事件: NodeCallback)
    RQ->>WW: 10. 唤醒 Workflow Worker
    WW->>WW: 11. 路由给对应 Plugin 的 handle_callback 吸收执行结果
    WW->>DB: 12. 更新 Node 状态并持久化，继续执行下一节点
```

### 1.3 WorkflowInstance 状态机（统一语义）

为避免 `Suspended` 语义混淆（人工暂停 vs 技术性等待），状态机约束如下：

| 当前状态 | 触发事件 | 目标状态 | 说明 |
|------|------|------|------|
| `Pending` | Worker 拉起执行（Start） | `Running` | 进入执行态 |
| `Running` | 分发异步子任务后让出 CPU | `Await` | 系统等待回调，不是人工暂停 |
| `Running` | 审批/暂停节点进入等待 | `Suspended` | 人工介入或定时等待场景 |
| `Running` | 正常结束 | `Completed` | 终态 |
| `Running` | 执行失败 | `Failed` | 终态（可重试） |
| `Running` | 用户取消 | `Canceled` | 终态 |
| `Await` | 收到回调并恢复调度 | `Pending` | 统一回到安全边界，再由 Worker 重新进入 `Running` |
| `Await` | 用户取消 | `Canceled` | 终态 |
| `Suspended` | 用户恢复/审批通过 | `Pending` | 不允许直达 `Running`，避免无锁恢复风险 |
| `Suspended` | 用户取消 | `Canceled` | 终态 |
| `Failed` | 用户重试 | `Pending` | 重新调度 |

关键约束：

1. `Await -> Pending`（禁止 `Await -> Running`）
2. `Suspended -> Pending`（禁止 `Suspended -> Running`）
3. `Pending` 是统一安全边界，只有 Worker 持锁执行时才进入 `Running`

### 1.4 工作流/任务实例：重试、取消、级联通知与跳过节点（规划）

本节在 **§1.3 状态表** 与现有 `WorkflowInstanceStatus` / `TaskInstanceStatus` / `NodeExecutionStatus` 之上，约定**可落地方案**（含与当前实现的差距），用于统一后端、API 与前端产品行为。

#### 编排驱动的统一事件入口（定案：方案 A）

工作流 Worker **只做编排**；正常路径是执行器 / 子工作流完成后投递 **`ExecuteWorkflowJob { event: NodeCallback, ... }`**，经 **`process_workflow_job`** 进入各插件 **`handle_callback`**，再推进图与派发下一批任务。

**方案 A（已定）**：凡属于「**某节点在持久化侧已有终态结论，需与执行器回调走同一套合并与推进逻辑**」的人工操作（典型：**跳过节点**、以及需与回调同路径处理的其它节点级干预），在 **API 完成 CAS 落库** 之后，应 **`dispatch_workflow(ExecuteWorkflowJob { event: NodeCallback, ... })`**，**与 Task Worker 使用同一事件形态、同一 Worker 入口**，不得在 HTTP Handler 内直接跑引擎循环。

**与 `Start` 的分工**：**`WorkflowEvent::Start`** 仍用于实例处于 **`Pending`** 时**进入主循环**（创建后首次执行、`Failed` / `Suspended` 经 **retry / resume** 回到 `Pending` 后的**整实例再调度**）。`Start` 与 `NodeCallback` **并列**，均属 **Redis/Apalis 上的 `ExecuteWorkflowJob`**，仅事件变体不同；**禁止**在 API 线程内内联执行 `run_loop`。

#### 持久化与队列的两阶段（强制）

| 阶段 | 职责 | 说明 |
|------|------|------|
| **1. 持久化** | 状态机迁移 + 节点/图数据与执行器结果**对齐**（如 skip 写入 `Skipped` + `output`、`current_node`） | 可由独立 API 或单接口内**先**完成写库。 |
| **2. 投递** | **`dispatch_workflow`** 或 **`dispatch_task`** | 仅发 Job，**不**在 Handler 内执行编排逻辑。 |

单接口内允许 **「写库 → 投递」** 顺序实现，但**语义上**仍分两阶段；前端也可拆成两个 API。**禁止**绕过队列直接调插件。

#### 1.4.1 术语与目标

| 维度 | 说明 |
|------|------|
| **工作流实例** | **retry** / **resume**：`→ Pending` 后投递 **`Start`**（与创建后 **execute** 对称）。**跳过**等节点级结论：落库后投递 **`NodeCallback`（方案 A）**。**取消**：`Failed` / `Suspended` → `Canceled`（一般不再投递编排 Job；若需唤醒父链，另议）。 |
| **原子任务实例** | **retry** `Failed→Pending` 后由 **`ExecuteTaskJob`**（**execute**）触发；嵌在工作流内时，父侧延续见 §1.4.4。 |
| **级联** | 父/子状态在**阶段 1** 修正后，**阶段 2** 对父投递 **`NodeCallback` 或 `Start`**（与执行器回调同级），保持「**事件流转通知工作流**」一致。 |
| **跳过节点** | 仅在 **§1.4.5** 安全窗口内；**`output` 由用户填写，允许 `{}`**，见下。 |

#### 1.4.2 工作流实例：后端能力（与 §1.3 对齐）

| 操作 | 状态前置 | 持久化 | 阶段 2 投递（约定） |
|------|----------|--------|---------------------|
| **execute** | `Pending` | `→ Running` | **`WorkflowEvent::Start`**（已有）。 |
| **retry** | `Failed` | `→ Pending` | **`Start`**（整实例从安全边界再跑）。 |
| **resume** | `Suspended` | `→ Pending` | **`Start`**。 |
| **cancel** | `Failed` / `Suspended` | `→ Canceled` | 通常无；若需通知父子链，可用 **`NodeCallback`** 表达终态（实现期定）。 |

**`Await` 下用户取消**：§1.3 表已列；若实现，需扩展状态机，并对滞留队列回调 **幂等丢弃**。

#### 1.4.3 原子任务实例：后端能力

| 操作 | 前置 | 持久化 | 阶段 2 |
|------|------|--------|--------|
| **execute** | `Pending` | `→ Running` | **`dispatch_task`**。 |
| **retry** | `Failed` | `→ Pending` | 由 **execute** 或级联后的父 **`Start`/`NodeCallback`** 触发子任务再投；级联写库见 §1.4.4。 |
| **cancel** | `Pending` / `Failed` | `→ Canceled` | 一般无 Task Job。 |

#### 1.4.4 `NodeCallback`（方案 A）字段约定与级联

与 **§1.2 架构交互图**、`ExecuteWorkflowJob` / `WorkflowEvent` 一致，人工注入与 Task Worker **共用**下列字段语义（实现上 **`handle_callback` 须幂等**，重复投递不破坏状态）：

| 字段 | 约定 |
|------|------|
| `node_id` | 工作流图中节点；跳过 / 重放结论所指向的节点。 |
| `child_task_id` | **优先**使用该节点当前嵌套 **`task_instance.task_instance_id`**，与真实回调对齐，便于插件复用合并逻辑；若尚无任务行，可用租户内约定占位 id，插件需分支处理。 |
| `status` | `NodeExecutionStatus`；**跳过**建议 **`Skipped`**（或 **`Success`** + `output` 与 **`Skipped` 节点态** 二选一，全项目统一一种）。 |
| `output` / `error_message` / `input` | 与 **阶段 1 已写入 DB** 的内容一致，避免回调与库双源不一致。 |

**级联（任务实例页重试、子工作流等）**：

1. **阶段 1**：按失败点修正 **子 `task_instance`、父 `WorkflowNodeInstanceEntity`、父 `WorkflowInstanceEntity`**（如父 `Failed→Pending`、`current_node` 等）。  
2. **阶段 2**：对**应继续的实例** `dispatch_workflow`：**父** 上投递 **`Start`**（若整父已 Pending）或 **`NodeCallback`**（若等价于「某父节点收到子结果」）；**子** 再跑则 **`dispatch_task`**。  
3. **子工作流完成通知父**：仍走现有 **子终态 → `NodeCallback` 父**（不变）。  

**禁止**：在 API 中伪造不完整 `NodeCallback` 导致 `handle_callback` 与锁/epoch/子任务 id 冲突；**必须**与持久化视图一致。

#### 1.4.5 跳过节点（Skip）

##### 1.4.5.1 核心原则

> **跳过必须做到"原子"级别**。用户跳过的目标永远是最内层的具体执行单元（Http、gRPC、Approval 等），而非容器本身。

- 对 **普通节点**（Http/gRPC/Approval/IfCondition/ContextRewrite）：直接跳过，与现有逻辑一致。
- 对 **Parallel/ForkJoin**：用户跳过的是容器内**某个失败的子任务**，而非整个容器节点。容器的状态机通过接收 `Skipped` 回调正常推进。
- 对 **SubWorkflow**：用户**直接操作子工作流实例**，对子工作流内部的具体失败节点执行 skip。子工作流完成后自然通过 `NodeCallback` 回调父工作流。
- 对 **Parallel/ForkJoin 内的 SubWorkflow 子任务**：行为与独立 SubWorkflow 一致 — 定位到子工作流实例，递归 skip 其内部节点。

**设计思路 B（已定）**：用户始终直接操作目标实例（无论是父还是子工作流），不在 API 层做自动递归。好处：
1. 用户从父工作流或子工作流进入，都能看到一致的视图
2. 正反两面路径一致 — 无论是正常回调还是人工 skip，都通过 `NodeCallback` 向上通知
3. 实现简单，复用现有 skip 逻辑，无需 API 层递归

##### 1.4.5.2 NodeExecutionStatus::Skipped 状态机

`Skipped` 是**终态**，与 `Success`、`Failed` 并列。允许的转入路径：

| 从状态 | 到状态 | 场景 |
|--------|--------|------|
| `Failed` | `Skipped` | 用户对失败节点执行 skip |
| `Suspended` | `Skipped` | 用户对挂起节点（如审批超时后）执行 skip |

**禁止**从 `Success`、`Pending`、`Running`、`Await` 直接转为 `Skipped`。`Skipped` 本身为终态，不可再转出。

对应 `TaskInstanceStatus`：跳过时将子任务状态设为 `Completed`（`TaskInstanceStatus` 无 `Skipped` 变体，复用 `Completed`）。

##### 1.4.5.3 普通节点跳过（已实现）

**API**：
```
POST /api/v1/workflow/instances/{id}/skip-node
Body: { "node_id": "node_3", "output": {} }
```

**`output`（统一字段，无额外别名）**

- Body 提供 **`node_id`** 与 **`output`**（JSON 对象）。**用户须提交 `output`，允许为 `{}`**。
- 持久化：节点 **`Skipped`**，该节点 **`task_instance.output = output`**（与成功节点一样占用同一字段，**不**引入 `synthetic_output` 等额外名）。
- **`build_nodes_object`**：`Skipped` 且 **`output` 已落盘**（含 `{}`）→ 生成 **`nodes.<id>.output`**，复用现有模板解析链。

**算法要点**：

1. **阶段 1**：校验租户与状态窗口；写 **`Skipped`** + **`output`**；实例回到 **`Pending`**。
2. **阶段 2**：**`dispatch_workflow(ExecuteWorkflowJob { event: NodeCallback, node_id, child_task_id, status: Skipped, output })`**，由 **`handle_callback`** 与执行器回调**同路径**推进。

**允许跳过的工作流实例状态**（安全窗口）：**`Failed`**、**`Suspended`**。

##### 1.4.5.4 Parallel / ForkJoin 子任务跳过

用户跳过的是容器内**某个具体失败的子任务**，而非容器节点整体。

**API**（扩展现有接口）：
```
POST /api/v1/workflow/instances/{id}/skip-node
Body: {
  "node_id": "node_6",                          // 容器节点 ID
  "child_task_id": "aee71fc7-...-node_6-2",    // 必须：具体子任务 ID
  "output": {}
}
```

**前置条件**：
- 工作流实例状态为 `Failed`（容器子任务有失败或触发 `max_failures` 提前终止）或 `Await`（子任务失败但未触发提前终止）
- `child_task_id` 对应的 `task_instances` 记录状态为 `Failed`
- `child_task_id` 必须属于 `node_id` 容器（格式校验：`{instance_id}-{node_id}-{index}`）

**执行流程**：

```
API: skip_node(instance_id, node_id="node_6", child_task_id="xxx-node_6-2", output={})
  │
  ├─ 阶段 1: 持久化
  │   ├─ 更新 task_instances 集合: child_task_id → task_status = Completed
  │   ├─ 不修改容器节点状态（Parallel 仍在 Await）
  │   ├─ 不修改 workflow_instance 状态
  │   └─ 若实例为 Failed（熔断后），转回 Await 以便接收回调
  │
  └─ 阶段 2: 投递
      └─ dispatch_workflow(NodeCallback {
           node_id: "node_6",
           child_task_id: "xxx-node_6-2",
           status: Skipped,
           output: user_output,
         })
```

**handle_callback 对 Skipped 的处理**：

Parallel / ForkJoin 的 `handle_callback` 将 `Skipped` **等同于 `Success` 处理**，同时记录独立计数：

```
# 首次回调（child_task_id 不在 processed_callbacks 中）
if status == Success → success_count += 1
if status == Skipped → success_count += 1, skipped_count += 1
if status == Failed  → failed_count += 1

# Skipped 覆盖（child_task_id 已在 processed_callbacks 中，但 status == Skipped）
从 results[child_task_id].status 读取原始状态
if 原始状态 == Failed  → failed_count -= 1
if 原始状态 == Success → success_count -= 1
然后: success_count += 1, skipped_count += 1
重新评估完成条件
```

> 非 Skipped 的重复回调仍被忽略（幂等保护）。仅 Skipped 允许覆盖，因为它是用户主动操作。

在容器状态机 `output` 中新增 `skipped_count` 字段和 `results` map（Parallel 与 ForkJoin 对称）：

**Parallel output 示例**（`results` key 为 `child_task_id`）：
```json
{
  "total_items": 10,
  "dispatched_count": 10,
  "success_count": 9,
  "failed_count": 0,
  "skipped_count": 1,
  "results": {
    "wf-node_6-0": { "status": "Skipped", "output": {}, "error": null },
    "wf-node_6-1": { "status": "Success", "output": { "..." }, "error": null }
  },
  "processed_callbacks": ["wf-node_6-0", "wf-node_6-1", ...]
}
```

**ForkJoin output 示例**（`results` key 为 `task_key`）：
```json
{
  "total_tasks": 3,
  "dispatched_count": 3,
  "success_count": 2,
  "failed_count": 0,
  "skipped_count": 1,
  "results": {
    "send_email": { "status": "Skipped", "output": {}, "error": null },
    "fetch_user": { "status": "Success", "output": { "..." }, "error": null }
  },
  "processed_callbacks": ["wf-fj-0", "wf-fj-1", ...]
}
```

完成检测条件不变：`success_count + failed_count == total_items`（其中 `success_count` 包含了被 skip 的任务）。

##### 1.4.5.5 SubWorkflow 节点跳过

SubWorkflow 节点**不直接跳过**。用户通过以下路径操作：

1. 在父工作流实例详情页，查看 SubWorkflow 节点的 `task_instance.output`，获取 `child_workflow_instance_id`
2. 导航到子工作流实例详情页
3. 对子工作流内部的具体失败节点（Http/gRPC 等）执行 skip
4. 子工作流恢复执行，到达终态后自动通过 `NodeCallback` 回调父工作流
5. 父工作流的 SubWorkflow 节点接收回调，正常推进

**Parallel/ForkJoin 内的 SubWorkflow 子任务**：行为一致。每个 SubWorkflow 子任务在 `execute` 阶段创建了独立的子工作流实例（`child_workflow_instance_id` 记录在该子任务的 output 中）。用户定位到对应的子工作流实例，递归 skip 其内部原子节点。

**此设计保证**：无论嵌套多深，用户始终操作最内层的原子节点，向上通知路径与正常执行完全一致。

##### 1.4.5.6 前端适配

| 页面 | 行为 |
|------|------|
| **普通节点详情** | 现有 skip 按钮，收集 `output`（默认 `{}`）后提交。 |
| **Parallel/ForkJoin 节点详情** | 展示子任务列表（从 `task_instances` 查询），每个 `Failed` 子任务旁显示 skip 按钮。提交时携带 `child_task_id`。 |
| **SubWorkflow 节点详情** | 展示 `child_workflow_instance_id` 链接，用户点击跳转到子工作流实例详情页操作。 |
| **工作流实例列表** | 无变化。 |

**审计（二期）**：`skipped_at`、`skipped_by`、`reason` 可选字段。

#### 1.4.6 前端（与 `docs/frontend-architecture.md` 对齐）

| 页面 | 行为 |
|------|------|
| **工作流实例** | **execute** 对应 **`Start`**；跳过在 UI 收集 **`output`（默认 `{}`）** 后提交；可提供「提交后由后端写库并投递」的单按钮。 |
| **任务实例** | 重试/级联后父侧依赖 **`Start`/`NodeCallback`** 的，提示与架构 §1.4.4 一致。 |

#### 1.4.7 实施阶段建议

| 阶段 | 内容 |
|------|------|
| **P0（已完成）** | **`build_nodes_object`** 支持 `Skipped`+`output`；普通节点 skip API **阶段1+2**；**`handle_callback`** 对人工 **`NodeCallback`** 幂等；retry/resume 后 **`Start`** 路径与现网对齐。 |
| **P1** | **容器 skip**：skip API 扩展 `child_task_id` 参数；Parallel/ForkJoin `handle_callback` 识别 `Skipped` 状态并记录 `skipped_count`；前端 Parallel/ForkJoin 节点详情展示子任务列表 + skip 按钮；SubWorkflow 节点详情展示子工作流实例跳转链接。 |
| **P2** | 任务页级联写库 + 父 **`Start`/`NodeCallback`**；`Await` 取消；队列补偿；审计（`skipped_at`/`skipped_by`/`reason`）。 |

---

## 2. 插件化系统 (Plugin System)

为了让系统拥有极强的扩展能力，我们将任务的抽象剥离成了两层接口：**编排插件接口 (`PluginInterface`)** 与 **执行器接口 (`TaskExecutor`)**。

### 2.1 编排插件 (`PluginInterface`)

该接口存在于 Workflow Worker 的上下文中。**它是工作流的路由大脑，而非干活的苦力。**
主要用于实现：如何发射子任务、遇到子任务回调时如何合并状态、如何进行分支跳转（如 If 条件）等控制流逻辑。

```rust
#[async_trait]
pub trait PluginInterface: Send + Sync {
    // 【前置逻辑】当工作流运行到该节点时触发。
    // 一般用于：解析配置、生成并向下分发 ExecuteTaskJob、或者针对控制流节点(IfCondition)直接返回跳到哪。
    async fn execute(...) -> anyhow::Result<ExecutionResult>;

    // 【后置回调逻辑】当远端的 Task Worker 汇报“任务已完成”时触发。
    // 一般用于：普通节点直接拷贝结果并完结；并发节点(Parallel)做数据聚合和增量派发。
    async fn handle_callback(...) -> anyhow::Result<ExecutionResult>;
    
    fn plugin_type(&self) -> TaskType;
}
```

**代表实现**：
* `HttpPlugin`：`execute` 里构造一条任务工单推到队列，直接返回 `Pending` 挂起。`handle_callback` 里直接接收结果，返回 `Success`。
* `IfConditionPlugin`：不需要队列交互。`execute` 里直接运行表达式库，算出来 `true` 还是 `false`，当即返回 `Success(JumpTo(TargetNode))` 告知工作流下一跳转去哪里。
* `ParallelPlugin`：并发大杀器，接下来详述。

### 2.2 任务执行器 (`TaskExecutor`)

该接口存在于 Task Worker 的上下文中。它的职责极其单一：拿到一段静态的输入参数，把它跑完，返回一段静态的输出结果。

```rust
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute_task(&self, task_instance: &TaskInstanceEntity) -> anyhow::Result<TaskExecutionResult>;
    fn task_type(&self) -> TaskType;
}
```

---

## 3. 并发容器 (Parallel Plugin) 深度剖析

`Parallel` 并不是一个普通的“执行某段代码”的节点，它是一个 **Scatter-Gather (分散-聚合)** 模式的微型调度器本身。
由于它可能需要遍历处理 10 万条数据的数组，因此**坚决不能**在一次 `execute` 里把 10 万条队列任务全部发出，这会瞬间造成内存 OOM 和 Redis 拥堵。

因此，Parallel 借助了我们的 `NodeCallback` 内部事件机制，实现了一个非常精妙的状态机。

### 3.1 状态机存储

Parallel 借用了它自身的 `TaskInstanceEntity.output` 来充当状态机的内存条，并利用 MongoDB 作为持久化存储（即便引擎宕机重启，进度也能从 DB 恢复）。
```json
{
  "total_items": 1000,
  "dispatched_count": 10,
  "success_count": 0,
  "failed_count": 0
}
```

### 3.2 增量控制流 (Scatter & Gather)

1. **首次执行 (Scatter 播种)**
   在 `ParallelPlugin::execute` 中，只解析目标数组大小（如 1000）。并且**仅仅**根据设定的 `concurrency` 阈值（如 10），生成**前 10 个** `ExecuteTaskJob` 扔进任务队列，并记录 `dispatched_count = 10`。
   > **关键点**：这发出去的 10 个 Job 中，它的 `caller_context` 里携带了 `parent_task_instance_id` (指向这个 Parallel 的 ID)，以及自己代表的是数组里的第 `item_index` 个元素。

2. **异步回调与增量补货 (Gather 收获)**
   随着 Task Worker 并发执行这 10 个任务，会有捷足先登者完成并向 Workflow Worker 发射 `NodeCallback` 事件。
   事件流转进入 `ParallelPlugin::handle_callback`，在此进行如下决断：
   
   * **状态聚合**：根据子任务的成败，对 `success_count` 或 `failed_count` 进行加一。
   * **提前终止**：`max_failures` 为**提前终止阈值**（非容忍阈值）。当 `failed_count >= max_failures` 时，整个 Parallel 立即短路失败，不再浪费资源执行剩余任务。若 `max_failures` 为 `None`，则不提前终止，等全部子任务完成后再判断。
   * **完成检测**：若 `success_count + failed_count == total_items`（全员收工），判定最终状态：`failed_count > 0` 则容器 **Failed**（无论 `max_failures` 设为多少），否则 **Success**。`max_failures` 仅控制提前终止时机，不改变最终成败判定。
   * **并发补货**：如果既未熔断也未完成，则根据并发模式开始派发新任务：
     * **Rolling (滚动模式)**：犹如滑动窗口，走了一个补一个，立即派发第 `dispatched_count` 号任务给队列，保证全速满负荷运转。
     * **Batch (批量模式)**：按兵不动，直到前 10 个任务全部死活出结果了，才一次性把后 10 个任务发出。

通过这种“任务唤醒自己”的设计（Event-Driven Callback），系统不仅做到了完美解耦，而且拥有了抵抗海量数据冲击的弹性能力。

### 3.3 节点可观测性（单一数据源）

工作流节点**不再**使用 `WorkflowNodeInstanceEntity.output`。节点在运行时的**入参与出参**一律落在嵌套的 `TaskInstanceEntity` 上：

| 字段 | 含义 |
|------|------|
| `task_instance.input` | **解析后**的执行入参快照。对 HTTP 任务：`url` 与 `headers`/`body`/`form` 各行按下文规则解析后写入；其它插件各自定义形状。供排障与审计；各插件在 `execute` / 默认或自定义 `handle_callback` 中写入。 |
| `task_instance.output` | 节点/任务对外结果（HTTP 响应摘要、If 分支结果、Parallel 父节点状态 JSON、ContextRewrite 的 key 列表等）。 |

**HTTP**：Workflow Worker 在 `run_node` 合并变量后调用 `resolved_http_request_snapshot` 写入**嵌套在流程实例节点上的** `task_instance.input`；投递异步任务前，`ensure_task_instance_for_job` 在 `task_instances` 集合中创建（或复用）与 `task_instance_id = {workflow_instance_id}-{node_id}` 对应的行。**图上的 HTTP 节点**必须把同一份已解析 `input` 写入该行：任务 Worker 只读 `task_instances`，若此处 `input` 为空，`HttpTaskExecutor` 会用空上下文重算，导致 `Variable` 模板中的 `{{}}` 无法替换。Parallel/ForkJoin 子任务在该函数内按 `item_alias` + `items_path` 或父节点 `context` 生成解析后的 `input`。执行器优先使用快照发请求，并在回调中回传同一快照。**禁止**在节点层再维护一份 `output`，避免双写不一致。

**HTTP 模板中 `FormValueType`（`headers` / `body` / `form` 每一行）**（实现：`domain::task::http_template_resolve`）：

- **任务级 `form` 与 `url`/`headers`/`body` 的先后**：先将模板 **`form` 数组**的每一行按 **base 上下文**（租户 / 工作流 / 实例 / 节点合并结果，见 §8.5–8.6）解析为键值对象，再将其 **覆盖合并** 到 base 上得到 **effective_ctx**（**同名键以任务 `form` 为准**）。**`url`、`headers`、`body`** 中的占位符与 `Variable` 行均按 **effective_ctx** 解析。快照 JSON 里的 **`form` 字段**仍为各 `form` 行在 **base 上下文**下的解析结果（任务级绑定层本身），便于对照设计态默认值。
- **`String` / `Number` / `Bool` / `Json`**：字面量。字符串中的 `{{...}}` **不会**被解析，便于需要原样传输占位符文案的场景。
- **`Variable`**：对字符串值做 **`{{点路径}}` 模板替换**（可整段为 `{{name}}`，也可混写为 `prefix {{name}} suffix`）；查找路径在 **effective_ctx** 上解析（故可由任务 `form` 提供 `body` 中引用的键）。
- **`url` 字段**（非表单行）：整段 URL 对 **effective_ctx** 做 `{{点路径}}` 占位符解析。

**破坏性说明**：曾依赖「`type: String` + 值里写 `{{key}}`」实现动态 Headers/Body 的旧数据，需改为 **`type: Variable`** 后才会继续解析。

**脱敏**：生产环境若需对 Secret 打码，在写入 `input`/`output` 前增加策略层（当前迭代未实现）。

---

## 4. 分布式锁与数据一致性 (CAS)

在分布式环境中，同一个工作流实例可能因为并发的回调（如 Parallel 节点的多个子任务同时完成）被多个 Workflow Worker 同时唤醒。为了防止“更新丢失（Lost Update）”和“数据撕裂”，引擎在 `WorkflowInstanceEntity` 中设计了乐观锁与租约机制：

```rust
pub struct WorkflowInstanceEntity {
    // ...
    pub epoch: u64, // 乐观锁版本号
    pub locked_by: Option<String>, // 当前持有该工作流锁的 Worker ID
    pub locked_duration: Option<std::time::Duration>, // 锁的过期时间（租约）
}
```

### 4.1 字段设计与租约 (Lease) 机制

* **`epoch` (乐观锁版本号)**：每次对 `WorkflowInstanceEntity` 的成功修改并持久化到 MongoDB 时，`epoch` 都会原子性加 1。任何 Worker 在更新时都必须携带自己读取到的 `epoch` 进行 CAS（Compare-And-Swap）操作。若数据库中实际 `epoch` 不匹配，说明数据已被其他 Worker 修改，当前更新操作被拒绝并触发重试。
* **`locked_by` 和 `locked_duration` (悲观租约)**：为了防止多个 Workflow Worker 频繁争抢同一个工作流实例导致的 CAS 冲突风暴（尤其在 Parallel 密集回调时），引擎引入了“租约”概念。当一个 Worker 认领了工作流事件，它会设置 `locked_by = "self_worker_id"` 并给予一定的锁定时长（如 10 秒）。
  * 在租约期间内，其他 Worker 哪怕收到了唤醒该工作流的事件，也需要让步或等待。
  * 如果持有锁的 Worker 崩溃宕机，租约（`locked_duration`）到期后，“扫地僧”机制或其他 Worker 即可重新抢占该实例，保证不会产生永远卡死在 Running 状态的孤儿任务。

### 4.2 并发容器 (Parallel) 回调的 CAS 演进过程

在 Parallel 节点运行中，假设有 100 个 HTTP 子任务被并发扔到 Task Worker 执行。随着任务快速完成，它们会密集地向 Workflow Worker 推送 `NodeCallback` 唤醒事件。

在这个场景下，数据的流转与 `epoch` 的变化如下：

1. **并发唤醒冲突**：子任务 A 和子任务 B 几乎同时完成，向队列中推入了两个 `NodeCallback` 事件。
2. **锁争抢阶段**：
   * Worker 1 消费到 A 的事件，Worker 2 消费到 B 的事件。
   * 它们同时去 MongoDB 读取 `WorkflowInstance`（当前 `epoch = 5`，无锁）。
   * Worker 1 和 Worker 2 尝试通过 CAS (条件 `epoch == 5` 且未被锁定) 抢占这把锁并更新。
   * MongoDB 的原子性保证了只有一个能赢，假设 Worker 1 赢了，此时 MongoDB 里 `locked_by = "Worker-1"`, `epoch` 变为 `6`。
3. **退让与事件不丢失 (Retry机制)**：
   * Worker 2 更新失败（受制于 CAS 检查 `epoch == 5` 失败，或发现被锁定），它会抛出 `OptimisticLockError` 或者 `LeaseLockedError`。
   * **如何确保事件不丢失**：因为底层的任务队列（Apalis/Redis）支持 **ACK/NACK 机制**。Worker 2 在处理该 `NodeCallback` 事件时一旦遇到数据库层面的乐观锁失败报错，它**不会向队列发送 ACK**。相反，它会向队列发送一个 NACK（或者让事件超时），这导致该 `NodeCallback` 事件会重新回到队列中（或进入重试死信队列），并在短暂的延迟后被 Worker 2 或其他 Worker 重新消费。
4. **安全处理状态**：
   * Worker 1 在租约保护下，安全地进入 `ParallelPlugin::handle_callback`，将状态机中的 `success_count` +1。
   * 处理完毕后，Worker 1 执行 Save 释放锁，此时 CAS 更新条件为 `epoch == 6`，更新成功后释放 `locked_by`，`epoch` 变为 `7`。
5. **后续唤醒**：
   * Worker 2 的重试机制再次触发，重新从 MongoDB 加载最新的状态（此时 `epoch = 7`，`success_count` 已经被加过 1 了）。
   * Worker 2 抢占成功（`epoch` 变 8），处理子任务 B 的回调，再次把 `success_count` +1，完成后释放，`epoch` 变 9。

**结论**：在密集回调的 Gather 阶段，虽然大量的子任务完成事件如洪水般涌向 Workflow Worker，但依靠 `epoch` (CAS乐观锁) 保证了 `success_count` 等共享数据绝对不会被并发覆盖；同时依靠 `locked_by` (租约) 减缓了冲突摩擦，保障了系统在极致并发下的数据强一致性。

---

## 5. 异构并发容器 (ForkJoin Plugin) 深度剖析

### 5.1 Parallel vs ForkJoin

| 维度 | Parallel (同构并发/数据并行) | ForkJoin (异构并发/任务并行) |
|------|------|------|
| 数据源 | 一个 JSON 数组 × 同一个任务模板 | N 个**不同的** TaskTemplate |
| 子任务类型 | 全部相同（如全是 HTTP） | 可以不同（HTTP + gRPC + ...） |
| 典型场景 | "给 1000 个用户各发一封邮件" | "同时拉用户数据 + 发通知 + 生成报告" |
| 并发控制 | concurrency + Rolling/Batch | 同样 concurrency + Rolling/Batch |
| 失败控制 | max_failures（提前终止阈值） | 同样 max_failures |

二者本质上共享同一套 Scatter-Gather + Callback 状态机，区别仅在于数据源：Parallel 从数组动态展开 N 个相同任务，ForkJoin 从静态列表展开 N 个不同任务。

### 5.2 模板定义

```rust
pub struct ForkJoinTemplate {
    pub tasks: Vec<ForkJoinTaskItem>,   // 子任务列表
    pub concurrency: u32,               // 并发度
    pub mode: ParallelMode,             // Rolling / Batch
    pub max_failures: Option<u32>,      // 提前终止阈值（failed >= n 时中止，None 不提前终止）
}

pub struct ForkJoinTaskItem {
    pub task_key: String,               // 容器内唯一标识 (results map 索引键)
    pub task_id: Option<String>,        // 关联的 TaskEntity ID (仅独立任务如 Http/gRPC)
    pub name: String,                   // 可读名称
    pub task_template: TaskTemplate,    // 任意原子任务模板 (Http, gRPC, ...)
}
```

### 5.3 子任务类型约束

由于引擎采用 Workflow Worker (编排) / Task Worker (执行) 双角色分离架构，容器类插件 (`Parallel`, `ForkJoin`) 和控制流插件 (`IfCondition`) 仅有 `PluginInterface` 实现（编排侧），没有 `TaskExecutor` 实现（执行侧）。

因此：**容器的子任务只能是拥有 `TaskExecutor` 的原子任务类型**（Http, gRPC, Approval 等）。

```
ForkJoin.execute()
  → 派发子任务 ExecuteTaskJob(template=Parallel)
  → Task Worker 消费
  → TaskManager 找不到 Parallel 的 TaskExecutor ❌
```

如需嵌套容器逻辑，正确的做法是将内层容器建模为一个**子工作流**，由 Workflow Worker 独立编排。

### 5.4 状态机存储

与 Parallel 一样借用 `TaskInstanceEntity.output`，但额外包含 `results` Map 用于按 `task_key` 索引每个子任务的执行结果：

```json
{
  "total_tasks": 3,
  "dispatched_count": 3,
  "success_count": 1,
  "failed_count": 0,
  "results": {
    "fetch_user": { "status": "Success", "output": { ... } },
    "send_email": { "status": "Failed", "error": "timeout" },
    "notify_slack": null
  }
}
```

后续节点可通过 `task_key` 精确引用某个子任务的输出结果。

#### task_id 与 task_key 的区分

- **`task_key`**：容器内唯一标识，作为 `results` map 的键，供后续节点引用子任务输出。值为人类可读标签（如 `"create_user"`）。
- **`task_id`**：可选，关联独立注册的 `TaskEntity` 记录（UUID）。仅对 Http、gRPC 等可独立存在的原子任务有意义；对工作流内部插件（IfCondition、ContextRewrite 等）或子工作流（已有 `SubWorkflowTemplate.workflow_meta_id`）无需填写。子任务实例化时，`task_id` 会传播到 `TaskInstanceEntity.task_id`，实现来源追溯。

Parallel 节点的 `task_id` 追溯通过 `WorkflowNodeEntity.task_id` 实现（编排时由客户端填写），因为 Parallel 内部仅有一种同构任务模板。

### 5.5 Scatter & Gather 流程

**Scatter (execute)**：读取 `tasks` 列表 → 初始化状态机 → 按 `concurrency` 派发前 N 个子任务 → `caller_context.item_index` 记录子任务在列表中的索引。

**Gather (handle_callback)**：与 Parallel 共享完全相同的决策逻辑（状态聚合 → 提前终止 → 完成检测 → 并发补货），唯一区别是额外将结果写入 `results[task_key]`。

---

## 6. 子工作流嵌套 (SubWorkflow Plugin)

工作流之间没有高低之分，任何工作流都可以作为另一个工作流中的一个节点被调用。SubWorkflow 节点本质上与 Http、Parallel 处于同一层次——都是工作流图中的一个节点。区别仅在于：Http 投递任务给 Task Worker，而 SubWorkflow 投递的是**一个全新的工作流实例**给 Workflow Worker。

### 6.1 模板定义

```rust
pub struct SubWorkflowTemplate {
    pub workflow_meta_id: String,    // 引用哪个工作流
    pub workflow_version: u32,       // 指定版本
    pub form: Vec<Form>,             // 传递给子工作流的初始上下文参数，与目标工作流 form 定义对齐
    pub timeout: Option<u64>,        // 超时（秒）
}
```

> `form` 替代了原 `input_mapping`。发起子工作流等同于发起工作流实例：指定 meta_id + version，填写 form 参数并**按存储的 `Form` 值合并进子实例初始上下文**（当前实现为序列化后的字面量合并，**不经** HTTP 任务同款 `Variable`/`{{}}` 解析）。若需从父上下文映射字段，应在编排或其它层显式建模。

### 6.2 实体扩展

`WorkflowInstanceEntity` 新增两个字段：

```rust
pub struct WorkflowInstanceEntity {
    // ... 原有字段 ...
    pub parent_context: Option<WorkflowCallerContext>, // 若自己是子工作流，记录父工作流信息
    pub depth: u32,                                    // 嵌套深度，根工作流=0
}
```

### 6.3 执行时序

```mermaid
sequenceDiagram
    participant PWW as Parent Workflow Worker
    participant RQ as Redis Queue
    participant CWW as Child Workflow Worker
    participant DB as MongoDB

    Note over PWW, DB: 1. 父工作流命中 SubWorkflow 节点
    PWW->>DB: 1. 加载子工作流模板 (WorkflowDefinitionService)
    PWW->>DB: 2. 创建子 WorkflowInstance (Pending, depth=parent+1, parent_context 指向父)
    PWW->>RQ: 3. 投递 ExecuteWorkflowJob(Start) 给子工作流
    PWW->>DB: 4. 父节点状态 → Suspended

    Note over RQ, DB: 2. 子工作流独立执行
    RQ->>CWW: 5. 消费子工作流 Start 事件
    CWW->>CWW: 6. 子工作流正常编排（Http/Parallel/ForkJoin/...）
    CWW->>DB: 7. 子工作流执行完毕 → Completed

    Note over CWW, PWW: 3. 子工作流完成 → 回调父工作流
    CWW->>CWW: 8. 检测到 parent_context 非空
    CWW->>RQ: 9. 投递 NodeCallback 给父工作流
    RQ->>PWW: 10. 父 Workflow Worker 唤醒
    PWW->>PWW: 11. handle_callback 接收子结果
    PWW->>DB: 12. 父节点 Success/Failed，继续下一节点
```

### 6.4 父回调机制

子工作流到达终态（Completed / Failed）后，`PluginManager::process_workflow_job` 在释放锁之前检查 `parent_context`：

* 若 `parent_context` 非空 → 向父工作流投递 `NodeCallback` 事件（携带子工作流的上下文作为 output）
* 若为空 → 根工作流，正常结束

这与 Task Worker 完成后回调父工作流的机制**完全一致**，复用了同一套 `NodeCallback` 事件通道。

### 6.5 防循环递归

| 策略 | 说明 |
|------|------|
| 深度限制 | `depth` 字段每嵌套一层 +1，超过 `MAX_DEPTH`（默认 10）直接拒绝 |

子工作流的 `handle_callback` 直接使用 `PluginInterface` trait 的默认实现——子工作流的完成状态和输出直接映射为父节点的状态。

---

## 7. 多租户系统 (Multi-Tenancy)

### 7.1 设计总则

系统从数据模型层面原生支持多租户，所有工作流和原子任务均携带 `tenant_id`。租户隔离的**唯一门禁**在 API Server 侧，执行引擎（Worker）对租户完全无感知，只按 Job 维度执行。

```
请求 → [AuthMiddleware] → [TenantGuard] → [PermissionGuard] → Handler
         ↓                    ↓                 ↓
    验证 JWT Token       校验租户状态         检查角色权限
    解析 AuthContext      注入 tenant_id      匹配路由所需权限
```

### 7.2 核心实体

**租户表 (TenantEntity)**

| 字段 | 类型 | 说明 |
|------|------|------|
| tenant_id | String | 主键 |
| name | String | 租户名称 |
| description | String | 描述 |
| status | TenantStatus | Active / Suspended / Deleted |
| max_workflows | Option\<u32\> | 配额：最大工作流模板数 |
| max_instances | Option\<u32\> | 配额：最大运行实例数 |

**用户表 (UserEntity)**

| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | String | 主键 |
| username | String | 唯一 |
| email | String | 唯一 |
| password_hash | String | bcrypt 哈希 |
| is_super_admin | bool | 系统级超管标记 |
| status | UserStatus | Active / Disabled |

**用户-租户角色关联表 (UserTenantRole)**

| 字段 | 类型 | 说明 |
|------|------|------|
| user_id | String | 外键 |
| tenant_id | String | 外键 |
| role | TenantRole | TenantAdmin / Developer / Operator / Viewer |

### 7.3 角色与权限矩阵

| 权限域 | SuperAdmin | TenantAdmin | Developer | Operator | Viewer |
|--------|:---:|:---:|:---:|:---:|:---:|
| 租户管理（创建/暂停/删除） | ✅ | ❌ | ❌ | ❌ | ❌ |
| 跨租户访问 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 租户内用户管理 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 工作流/任务模板 CRUD | ✅ | ✅ | ✅ | ❌ | ❌ |
| 实例 创建/执行/取消/重试 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 只读查看 | ✅ | ✅ | ✅ | ✅ | ✅ |

### 7.4 JWT Token 设计

```json
{
  "sub": "user_id",
  "username": "alice",
  "is_super_admin": false,
  "tenant_id": "tenant_001",
  "role": "Developer",
  "exp": 1700000000
}
```

SuperAdmin 可通过 `X-Tenant-Id` Header 代入任意租户上下文。

### 7.5 数据隔离

所有 Repository 查询自动携带 `tenant_id` 过滤条件。Handler 从 `AuthContext` 获取 `tenant_id` 传递给 Service/Repository 层，确保数据不会跨租户泄漏。

### 7.6 API 路由

```
/api/v1/auth             ← 公开（无需 JWT）
  POST /login
  POST /register

/api/v1/tenants          ← SuperAdmin 专属
  CRUD

/api/v1/tenants/users    ← TenantAdmin+
  邀请/移除/改角色

/api/v1/workflow         ← 以下全部 tenant_id 自动隔离
/api/v1/task
```

---

## 8. 变量系统 (Variable System)

### 8.1 设计总则

变量系统为工作流引擎提供了统一的外部配置注入能力。用户可以在不修改工作流模板的前提下，通过变量来改变运行时行为（如切换 API 地址、注入凭证等）。变量按作用域分层，越靠近执行现场的变量优先级越高。

### 8.2 变量层级与优先级

| 优先级 | 层级 | scope_id | 说明 | 来源 |
|:------:|------|----------|------|------|
| 1 (最低) | **租户变量** | `tenant_id` | 全租户共享的基础配置 | `VariableEntity (scope=Tenant)` |
| 2 | **工作流模板变量** | `workflow_meta_id` | 某个工作流的默认配置 | `VariableEntity (scope=WorkflowMeta)` |
| 3 | **工作流实例变量** | `workflow_instance_id` | 运行时传入的上下文 | `WorkflowInstanceEntity.context` |
| 4 | **节点上下文** | `node_id` | 单节点的局部覆盖（模板拷贝 + 运行期合并结果） | `WorkflowNodeInstanceEntity.context`（初值来自 `WorkflowNodeEntity.context`） |
| 5 (执行期最高) | **系统 `nodes`** | — | 仅保留字顶层键 `nodes`：已完成节点的输出引用 | 由引擎在 `resolve_variables` **之后**注入，**覆盖**合并结果中与 `nodes` 同名的用户键 |

> 优先级 1～4 由 `VariableService::resolve_variables` 合并；**越靠近执行代码越高**。优先级 5 仅作用于顶层键 `nodes`：编排者不应在实例 `context` 中依赖自定义 `nodes` 键（会被系统覆盖）。变量系统为优先级 1、2 新增持久化实体；3、4 来自现有实体字段。

### 8.3 变量实体

```rust
pub enum VariableScope {
    Tenant,        // 租户级
    WorkflowMeta,  // 工作流模板级
}

pub enum VariableType {
    String,
    Number,
    Bool,
    Json,
    Secret,  // 加密存储，API 掩码返回
}

pub struct VariableEntity {
    pub id: String,
    pub tenant_id: String,
    pub scope: VariableScope,
    pub scope_id: String,          // tenant_id 或 workflow_meta_id
    pub key: String,               // 变量名，scope_id + key 唯一
    pub value: String,             // 存储值（Secret 类型 AES-256-GCM 加密）
    pub variable_type: VariableType,
    pub description: Option<String>,
    pub created_by: String,        // user_id
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 8.4 变量类型与安全策略

| 类型 | 存储方式 | API 响应 | 引擎解析 |
|------|----------|----------|----------|
| `String` | 明文 | 明文 | 原样注入 |
| `Number` | 明文 | 明文 | 解析为 `f64` |
| `Bool` | 明文 | 明文 | 解析为 `bool` |
| `Json` | 明文 JSON | 明文 | 解析为 `serde_json::Value` |
| `Secret` | **AES-256-GCM 加密** | `"******"` 掩码 | **运行时解密注入，不落盘到 output** |

加密密钥来源于环境变量 `VARIABLE_ENCRYPT_KEY`，缺失时拒绝启动。

### 8.5 变量解析流程

引擎在执行每个节点前，由 `VariableService::resolve_variables` 构造合并后的变量上下文：

```
resolve_variables(tenant_id, workflow_meta_id, instance_context, node_context):
    merged = {}
    merged.extend(load_tenant_variables(tenant_id))          // 优先级 1
    merged.extend(load_workflow_meta_variables(meta_id))      // 优先级 2 覆盖 1
    merged.extend(instance_context)                           // 优先级 3 覆盖 2
    merged.extend(node_context)                               // 优先级 4 覆盖 3
    return merged
```

随后 `PluginManager::run_node` 调用 `augment_merged_context_with_nodes`，在合并结果上**写入/覆盖**顶层键 `nodes`（见下节）。**节点执行前、用于 Rhai 与 HTTP `Variable` 行 / `url` 字段中 `{{}}` 模板解析的最终 JSON** = `merged` + 系统 `nodes`。

Secret 类型的变量在解析阶段解密注入，但**绝不写入** `TaskInstanceEntity.output`，防止凭证泄漏到持久化层。

### 8.6 执行期节点输出命名空间 (`nodes`)

引擎在**每个节点**进入插件逻辑前，在 `resolve_variables` 产物之上注入只读结构的顶层键 `nodes`，供 **IfCondition、ContextRewrite（`ctx`）、HTTP 任务中 `Variable` 类型表单行与 `url` 字段的 `{{path}}`、Parallel 的 `items_path`（经 `WorkflowNodeInstanceEntity.context` 解析）** 等统一引用。

| 规则 | 说明 |
|------|------|
| 形状 | `nodes.<node_id>.output` → 与该 `node_id` 对应的 `TaskInstanceEntity.output`（JSON） |
| 收录条件 | 仅 **非当前节点**、**`task_instance.output` 已落盘**，且节点为 **`Success`**，或 **`Skipped`**（用户填写的 **`output`**，可为 `{}`，见 **§1.4.5**）的图节点 |
| 当前节点 | **不包含**当前正在执行的节点（执行前尚无本次 output，避免自引用） |
| Parallel / ForkJoin | **子任务**（队列子 `TaskInstance`）**不**作为 `nodes` 中的独立条目；仅**容器节点**自身有一条 `nodes.<容器node_id>.output`（如状态机 JSON） |
| 优先级 | 系统构造的 `nodes` **覆盖**合并上下文中同名顶层键 `nodes`（用户不应占用该保留字） |
| 快照语义 | 与「节点级解析上下文」一致：`resolve_variables` 之后的合并结果再注入 `nodes`，再用于生成 `task_instance.input` 快照与 Rhai `ctx` |

实现位于 `domain/workflow/resolution_context.rs`（`build_nodes_object`、`augment_merged_context_with_nodes`）。**`Skipped` + `output`** 纳入 `nodes` 的规则以本文与 **§1.4.5** 为准；代码演进时需与上表「收录条件」对齐。Parallel/ForkJoin **内层** HTTP 子任务在 `ensure_task_instance_for_job` 中使用**父节点**已持久化的 `WorkflowNodeInstanceEntity.context` 作为模板解析基底，从而同样可引用 `nodes.*`。

### 8.7 权限矩阵

| 操作 | SuperAdmin | TenantAdmin | Developer | Operator | Viewer |
|------|:---:|:---:|:---:|:---:|:---:|
| 租户变量 CRUD | ✅ | ✅ | 仅读（Secret 掩码） | 仅读（Secret 掩码） | 仅读（Secret 掩码） |
| 租户 Secret 变量 创建/改值 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 模板变量 CRUD | ✅ | ✅ | ✅ | 仅读 | 仅读 |
| 模板 Secret 变量 创建/改值 | ✅ | ✅ | ✅ | ❌ | ❌ |

### 8.8 API 路由

```
# 租户变量
GET    /api/v1/variables                                    # 列表
POST   /api/v1/variables                                    # 创建
GET    /api/v1/variables/{id}                               # 详情（Secret 掩码）
PUT    /api/v1/variables/{id}                               # 更新
DELETE /api/v1/variables/{id}                               # 删除

# 工作流模板变量
GET    /api/v1/workflow/meta/{meta_id}/variables             # 列表
POST   /api/v1/workflow/meta/{meta_id}/variables             # 创建
GET    /api/v1/workflow/meta/{meta_id}/variables/{id}        # 详情
PUT    /api/v1/workflow/meta/{meta_id}/variables/{id}        # 更新
DELETE /api/v1/workflow/meta/{meta_id}/variables/{id}        # 删除
```

### 8.9 涉及代码变更

| 层级 | 变更 |
|------|------|
| `domain/variable/entity` | 新增 `VariableEntity`、`VariableScope`、`VariableType` |
| `domain/variable/repository` | `VariableRepository` trait |
| `domain/variable/service` | `VariableService`（CRUD + AES 加解密 + `resolve_variables`） |
| `domain/workflow/resolution_context` | `nodes` 注入与合并上下文增强 |
| `infrastructure/mongodb/variable` | MongoDB 实现 |
| `api/handler/variable` | Handler + 权限守卫 |
| `api/router` | 路由挂载 |
| `domain/plugin/manager` | `run_node`：`resolve_variables` 后 `augment_merged_context_with_nodes`；`ensure_task_instance_for_job` 内层 HTTP 使用父节点 `context` |

---

## 9. 上下文重写插件 (ContextRewrite Plugin)

### 9.1 定位

纯计算同步节点，运行在 Workflow Worker（与 IfCondition 同级），**不需要 Task Worker**。在两个节点之间插入一段 Rhai 脚本，对工作流上下文做任意变换，结果写回 `workflow_instance.context` 供后续节点使用。

典型场景：

| 场景 | 说明 |
|------|------|
| 字段提取 | HTTP 响应 `{"data":{"users":[...]}}` → 提取为 `{"users":[...]}` |
| 类型转换 | `"price":"99.5"` → `"price_num":99.5` |
| 数据聚合 | 将 Parallel/ForkJoin 的多个子结果合并为一个汇总对象 |
| 条件赋值 | 根据 `amount > 1000` 设置 `tier = "premium"` |
| 变量重命名/清理 | 去除敏感字段、统一命名规范后传递给下游节点 |

### 9.2 模板定义

```rust
pub struct ContextRewriteTemplate {
    pub name: String,
    pub script: String,          // Rhai 脚本，返回一个 Map
    pub merge_mode: MergeMode,   // Merge(增量覆盖) | Replace(完全替换)
}

pub enum MergeMode {
    Merge,    // 将脚本返回值 merge 进现有 context（默认）
    Replace,  // 用脚本返回值完全替换 context
}
```

### 9.3 Rhai 脚本规范

引擎将 **节点执行前解析上下文** 注入 Rhai 作用域为 `ctx`：即 `resolve_variables` 合并结果再经 **8.6 节** 注入顶层 `nodes` 后的 JSON（与 IfCondition、HTTP 模板同源）。脚本中可通过 `ctx["nodes"]["<node_id>"]["output"]` 读取已成功前置节点的落盘输出。脚本最后一行返回 `#{ ... }` Map 对象。

```rhai
// 示例：字段提取
let users = ctx["response"]["data"]["users"];
#{
    "user_count": users.len(),
    "user_names": users.map(|u| u.name)
}
```

```rhai
// 示例：条件赋值
let tier = if ctx["amount"] > 1000 { "premium" } else { "standard" };
#{
    "customer_tier": tier,
    "discount": if tier == "premium" { 0.2 } else { 0.0 }
}
```

### 9.4 执行流程

```
Workflow Worker 命中 ContextRewrite 节点
  ├─ 1. resolve_variables() 合并变量层级 → merged_ctx
  ├─ 2. augment_merged_context_with_nodes → 写入系统 `nodes`
  ├─ 3. 将上述 JSON 注入 Rhai Scope 为 `ctx`
  ├─ 4. 执行 script，获取返回的 Map
  ├─ 5. Merge 模式 → context.extend(result)
  │     Replace 模式 → context = result
  ├─ 6. 写回 workflow_instance.context
  └─ 7. 返回 ExecutionResult::success()，同步推进
```

### 9.5 安全约束

| 维度 | 策略 |
|------|------|
| 沙箱 | Rhai 内建沙箱，无文件/网络/系统调用能力 |
| 资源限制 | `Engine::set_max_operations(100_000)` 防止死循环 |
| 超时 | 脚本执行超过 5 秒中断，节点标记 Failed |
| Secret 变量 | 经 resolve_variables 解密注入到 ctx，但写回 context 前过滤掉 Secret 变量的 key |

### 9.6 共享 Rhai 引擎

ContextRewrite 与 IfCondition 共用 Rhai 引擎，提取为 `plugin/rhai_engine.rs` 共享模块，统一管理：
- Rhai `Engine` 实例创建与配置（operations limit）
- `ctx` 变量的注入逻辑
- JSON ↔ Rhai Dynamic 的类型转换

---

## 10. 审批插件 (Approval Plugin)

### 10.1 定位

审批节点是一个**纯 API 驱动的挂起节点**，不需要 Task Worker。工作流命中审批节点后挂起，等待人工通过 API 提交审批决策，决策完成后通过 `NodeCallback` 唤醒工作流。

### 10.2 模板定义

```rust
pub struct ApprovalTemplate {
    pub name: String,
    pub title: String,                       // 审批标题
    pub description: Option<String>,         // 审批说明
    pub approvers: Vec<ApproverRule>,        // 审批人规则
    pub approval_mode: ApprovalMode,         // Any / All / Majority
    pub timeout: Option<u64>,                // 超时秒数，None=不超时
}

pub enum ApproverRule {
    User(String),                            // 指定 user_id
    Role(TenantRole),                        // 指定角色
    ContextVariable(String),                 // 从上下文动态解析
}

pub enum ApprovalMode {
    Any,       // 任一审批人通过/拒绝即生效（抢单模式）
    All,       // 全部通过才通过，任一拒绝即驳回（会签模式）
    Majority,  // 多数通过即生效（投票模式）
}
```

### 10.3 审批实例实体

独立持久化，支持 "我的待办" 等列表查询场景。

```rust
pub struct ApprovalInstanceEntity {
    pub id: String,
    pub tenant_id: String,
    pub workflow_instance_id: String,
    pub node_id: String,
    pub title: String,
    pub description: Option<String>,
    pub approval_mode: ApprovalMode,
    pub approvers: Vec<String>,              // 解析后的 user_id 列表
    pub decisions: Vec<ApprovalDecision>,
    pub status: ApprovalStatus,              // Pending / Approved / Rejected
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct ApprovalDecision {
    pub user_id: String,
    pub decision: Decision,                  // Approve / Reject
    pub comment: Option<String>,
    pub decided_at: DateTime<Utc>,
}
```

### 10.4 执行时序

```mermaid
sequenceDiagram
    participant WW as Workflow Worker
    participant DB as MongoDB
    participant API as API Server
    participant User as 审批人
    participant RQ as Redis Queue

    Note over WW, DB: 1. 工作流命中审批节点
    WW->>DB: 创建 ApprovalInstance (status=Pending)
    WW->>DB: Node 状态 → Suspended
    WW->>WW: 释放锁，退出

    Note over User, API: 2. 人工审批
    User->>API: GET /approvals (查看待办)
    User->>API: POST /approvals/{id}/decide {Approve/Reject}
    API->>DB: 记录 ApprovalDecision
    API->>API: 判定模式条件 (Any/All/Majority)

    Note over API, WW: 3. 达成最终结果
    API->>DB: 更新 ApprovalInstance.status
    API->>RQ: 投递 NodeCallback 事件
    RQ->>WW: 唤醒 Workflow Worker
    WW->>WW: handle_callback → Approved=Success / Rejected=Failed
    WW->>DB: 更新节点状态，继续执行
```

### 10.5 审批模式判定逻辑

| 模式 | 通过条件 | 驳回条件 |
|------|----------|----------|
| `Any` | 第一个 Approve | 第一个 Reject |
| `All` | 全部 Approve | 任一 Reject |
| `Majority` | Approve 数 > 总数/2 | Reject 数 > 总数/2 |

### 10.6 API 路由

```
GET    /api/v1/approvals                     # 我的待审批列表
GET    /api/v1/approvals/{id}                # 审批详情
POST   /api/v1/approvals/{id}/decide         # 提交决策
```

### 10.7 权限

| 操作 | 限制 |
|------|------|
| 查看待审批 | 任何已认证用户（按 approvers 过滤） |
| 提交决策 | 必须在 approvers 列表内，且 Viewer 角色除外 |
| 查看所有审批 | TenantAdmin+ |

### 10.8 工作流发起人与审批申请人 (`created_by`)

#### 10.8.1 问题

当前 `WorkflowInstanceEntity` 和 `ApprovalInstanceEntity` 均无发起人/申请人字段。API 层虽然有 `auth.user_id`（JWT `sub`），但仅用于租户校验后即丢弃。导致：

- 审批通知无法发给"流程发起人"（§16.2.2 `approval.approved/rejected` 的推送目标 = "审批发起方"，当前无法定位）
- 无法实现自审批防护（不知道谁是申请人）
- 前端"我发起的"工作流查询无法实现
- 子工作流的发起人语义不明确

#### 10.8.2 实体变更

**`WorkflowInstanceEntity`**（`domain/workflow/entity/workflow_definition.rs`）新增：

```rust
#[serde(default)]
pub created_by: Option<String>,  // 发起人 user_id；None = 系统/定时触发
```

**`ApprovalInstanceEntity`**（`domain/approval/entity/mod.rs`）新增：

```rust
#[serde(default)]
pub applicant_id: Option<String>,  // 审批申请人 = workflow_instance.created_by
```

两个字段均使用 `Option<String>` + `#[serde(default)]`：
- MongoDB 中已有旧文档不含此字段，反序列化时 `default` 为 `None` 而非报错，**无需数据迁移**
- 系统自动触发的工作流（定时任务、事件驱动）可能没有人类发起人，`None` 准确表达此语义

#### 10.8.3 `create_instance` 签名变更

`WorkflowInstanceService::create_instance` 新增 `created_by: Option<String>` 参数。两个调用点的传值策略：

| 调用点 | `created_by` 值 | 说明 |
|--------|-----------------|------|
| HTTP Handler（`workflow_instance_handler.rs`） | `Some(auth.user_id.clone())` | 人类用户发起 |
| SubWorkflow Plugin（`subworkflow.rs`） | `workflow_instance.created_by.clone()` | 继承根发起人（见 §10.8.4） |

#### 10.8.4 子工作流继承策略（定案：继承根发起人）

```
用户 Alice 发起 W1 (根工作流)
  → W1 执行到 SubWorkflow 节点
    → 引擎内部调用 create_instance 创建 W2 (子工作流)
      → W2 中有审批节点
```

| 策略 | W2.created_by | 评估 |
|------|---------------|------|
| **A. 继承根发起人（定案）** | `Alice` | 业务语义最直观："我发起的"查询可追踪整条链路；自审批防护可生效 |
| B. 设为 None | `None` | 审批没有申请人，自审批策略失效，通知无目标 |
| C. 双字段 | 复杂度高 | 两个字段增加理解成本，收益不大 |

**选择 A 的理由**：

1. **审批语义正确**：子工作流中的审批**就是**根发起人的操作引发的，审批人看到"申请人：Alice"完全正确。
2. **自审批防护依赖此信息**：`applicant_id = None` 时自审批策略无法生效。
3. **父子关系已有独立追踪**：`WorkflowCallerContext`（`parent_context`）已记录父实例 ID 和触发节点，不需要 `created_by` 承担此职责。
4. **实现极简**：SubWorkflow 插件只需 `.clone()` 透传，无额外查询。

**边界情况**：事件/定时触发的根工作流 `created_by = None`，子工作流自然继承 `None`。此时审批的 `applicant_id` 为 `None`，前端可展示为"系统触发"，自审批策略因无 applicant 而不做过滤——这是正确行为。

**未来扩展点**：若出现"中间人触发"场景（如审批人批准后自动触发下一段流程，此时子流程发起人应为审批人而非根发起人），可在 `SubWorkflowTemplate` 上增加 `override_initiator` 配置项（`inherit` / `from_context_variable` / `none`），当前版本不需要。

#### 10.8.5 `ApprovalService::create_approval` 签名变更

新增 `applicant_id: Option<String>` 参数，写入 `ApprovalInstanceEntity.applicant_id`。

调用方 `ApprovalPlugin::execute` 从 `workflow_instance.created_by` 取值传入。

#### 10.8.6 系统变量 `__initiator__` 注入

在 `PluginManager::run_node`（`plugin/manager/workflow.rs`）中，`resolve_variables` 合并完成 **之后**、`augment_merged_context_with_nodes` **之前**，注入系统保留变量：

```rust
if let Some(ref uid) = instance.created_by {
    if let Some(obj) = node.context.as_object_mut() {
        obj.insert("__initiator__".into(), json!(uid));
    }
}
```

| 维度 | 说明 |
|------|------|
| 注入时机 | `resolve_variables` 之后，覆盖用户在 context 中伪造的同名 key |
| 命名约定 | `__` 前缀表示系统保留，不鼓励用户直接使用 |
| 优先级 | 高于 instance_context 和 node_context（§8.2），低于 `nodes`（§8.6） |
| 用途 | 审批模板的 `ApproverRule::ContextVariable("__initiator__")` 可引用发起人 |
| `created_by = None` 时 | 不注入 `__initiator__`，context 中无此 key |

#### 10.8.7 MongoDB 持久化

| 变更 | 说明 |
|------|------|
| `workflow_instances` collection | 文档自动包含 `created_by` 字段（serde 序列化） |
| `approval_instances` collection | 文档自动包含 `applicant_id` 字段 |
| `indexes.rs` 新增索引 | `workflow_instances: (tenant_id, created_by)` — 支持"我发起的"查询 |
| 旧数据兼容 | `serde(default)` → 旧文档读出 `None`，**无需数据迁移** |

#### 10.8.8 前端影响

| 层面 | 变更 |
|------|------|
| `types/workflow.d.ts` | `WorkflowInstanceEntity` 增加 `created_by?: string` |
| `types/approval.d.ts` | `ApprovalInstanceEntity` 增加 `applicant_id?: string` |
| 工作流实例列表页 | 可展示"发起人"列，可增加"我发起的"筛选 |
| 审批详情页 | 展示"申请人"信息 |
| 创建实例 | **无变更** — `created_by` 由服务端从 JWT 提取，前端不感知 |

#### 10.8.9 涉及文件清单

```
后端（必改）:
  domain/workflow/entity/workflow_definition.rs    # 实体 +created_by
  domain/workflow/service.rs                        # create_instance 签名 +created_by
  domain/approval/entity/mod.rs                     # 实体 +applicant_id
  domain/approval/service.rs                        # create_approval 签名 +applicant_id
  domain/plugin/plugins/approval.rs                 # 传入 workflow_instance.created_by
  domain/plugin/plugins/subworkflow.rs              # 透传 workflow_instance.created_by
  domain/plugin/manager/workflow.rs                 # run_node 注入 __initiator__
  api/handler/workflow/workflow_instance_handler.rs  # 传入 auth.user_id
  infrastructure/mongodb/indexes.rs                 # (tenant_id, created_by) 索引

后端（编译适配 — struct literal 补字段）:
  domain/plugin/plugins/parallel.rs                 # make_instance fixture
  domain/plugin/plugins/forkjoin.rs                 # make_instance fixture
  domain/workflow/resolution_context.rs              # 测试 fixture

前端:
  frontend/src/types/workflow.d.ts
  frontend/src/types/approval.d.ts
```

#### 10.8.10 为后续特性铺路

| 后续特性 | 依赖关系 |
|---------|---------|
| 自审批策略（§10.9） | `ApprovalInstanceEntity.applicant_id` + `resolve_approvers` 过滤 |
| 跳过审批的审计日志（§10.10） | `created_by` 用于"操作人"对比 |
| 通知发起人（§16.2.2） | `NotificationService` 从 `created_by` / `applicant_id` 获取目标用户 |
| "我发起的"列表 | `(tenant_id, created_by)` 索引支持查询 |

### 10.9 自审批策略 (Self-Approval Policy)（规划）

> 本节依赖 §10.8 `created_by` / `applicant_id` 先行落地。

#### 10.9.1 问题

当前 `ApprovalService::decide` 仅校验：用户在审批人列表中 + 未重复审批。如果工作流发起人碰巧也是审批人（通过角色匹配或指定用户），可以自己批准自己的申请。

#### 10.9.2 策略枚举

在 `ApprovalTemplate`（`task/entity/task_definition.rs`）上新增字段：

```rust
#[serde(default = "default_self_approval_policy")]
pub self_approval: SelfApprovalPolicy,

pub enum SelfApprovalPolicy {
    Allow,   // 允许自审批
    Skip,    // 从审批人列表中移除发起人（默认）
}

fn default_self_approval_policy() -> SelfApprovalPolicy {
    SelfApprovalPolicy::Skip
}
```

| 策略 | 行为 | 适用场景 |
|------|------|----------|
| `Skip`（默认） | `resolve_approvers` 阶段移除 `applicant_id` | 财务、合规、权限变更 |
| `Allow` | 不做过滤 | 低风险流程、测试环境 |

业界参考：Oracle Procurement 的 "Prohibit User Self-Approval" 配置；飞书审批的"发起人自动跳过"规则。

#### 10.9.3 执行逻辑

在 `ApprovalService::create_approval` 的 `resolve_approvers` 之后、空列表检查之前插入过滤：

```
resolve_approvers(rules, context) → user_ids
if policy == Skip && applicant_id ∈ user_ids:
    user_ids.remove(applicant_id)
if user_ids.is_empty():
    return Err("过滤自审批后无可用审批人")
```

**`applicant_id = None` 时**（系统触发）不做过滤，策略无效但不报错。

#### 10.9.4 前端

审批模板编辑器（任务模板 + 工作流节点编辑）增加"自审批策略"下拉：`跳过发起人`（默认）/ `允许`。

### 10.10 跳过审批节点的权限控制（规划）

> 本节是对 §1.4.5 跳过节点的审批特化补充。

#### 10.10.1 问题

当前 `skip_workflow_node` 允许任何有 API 权限的用户跳过审批节点。对于审批这类有合规含义的节点，应当增加额外的权限控制和审计。

#### 10.10.2 增强方案

| 维度 | 当前 | 增强 |
|------|------|------|
| **谁能跳过审批节点** | 任何有 API 权限的用户 | 限制为 TenantAdmin+，普通 Operator 不可跳过 Approval 类型节点 |
| **跳过理由** | 无 | `SkipNodeRequest` 增加 `reason: String` 必填字段（仅 Approval 节点强制） |
| **审计记录** | 无 | 跳过操作写入审计日志：操作人 + 理由 + 被跳过的节点 + 时间 |
| **超时策略** | 超时后 `Suspended`，需手动操作 | 模板可配置超时策略：`Suspend`（当前）/ `AutoSkip` / `AutoReject` |

超时策略扩展 `ApprovalTemplate`：

```rust
#[serde(default)]
pub timeout_action: ApprovalTimeoutAction,

pub enum ApprovalTimeoutAction {
    Suspend,     // 当前行为：挂起等人处理
    AutoReject,  // Sweeper 自动驳回（§13.11 已实现）
    AutoSkip,    // Sweeper 自动跳过并继续流程
}
```

---

## 11. 暂停节点 (Pause Plugin)

### 11.1 定位

暂停节点提供工作流执行过程中的**定时等待**能力。支持两种模式：

| 模式 | 行为 |
|------|------|
| **Auto（自动）** | 等待指定秒数后自动完成，工作流继续执行下一节点 |
| **Manual（手动）** | 等待指定秒数后进入等待确认状态，需人工点击"确认继续"后推进 |

暂停节点**不需要 Task Worker**，与 Approval、IfCondition 同级——纯编排侧插件，由 Sweeper 和 API 驱动完成。

### 11.2 模板定义

```rust
pub enum PauseMode {
    Auto,    // 到期后自动完成
    Manual,  // 到期后需人工确认
}

pub struct PauseTemplate {
    pub wait_seconds: u64,   // 等待秒数
    pub mode: PauseMode,
}
```

### 11.3 执行流程

```
Workflow Worker 命中 Pause 节点
  ├─ 1. 计算 resume_at = now + wait_seconds
  ├─ 2. 写入 node_instance.output = { mode, wait_seconds, resume_at }
  ├─ 3. 返回 ExecutionResult::suspended()
  └─ 4. 工作流实例状态 → Suspended
```

### 11.4 完成机制

#### Auto 模式 — Sweeper 自动唤醒

Sweeper 在每个扫描周期执行以下额外步骤：

```
sweep_expired_pause_nodes():
  1. 查询 status == Suspended 的工作流实例
  2. 遍历实例的节点，找到满足以下条件的节点:
     - node_type == Pause
     - node.status == Suspended
     - task_template.mode == Auto
     - output.resume_at <= now()
  3. 对匹配的节点，dispatch NodeCallback(Success)
```

**精度**：受 Sweeper 扫描间隔影响（默认 60s），实际恢复时间 = wait_seconds + 0~interval_secs。若需更高精度，可缩短 `sweeper.interval_secs`。

#### Manual 模式 — 人工确认 API

```
POST /api/v1/workflow/instance/{id}/resume-node
Body: { "node_id": "pause_1" }
```

**处理逻辑**：
1. 校验节点为 Pause 类型且 mode == Manual
2. 校验节点状态为 Suspended
3. **服务端强制校验**：`resume_at > now()` 时返回 400 "请等待倒计时结束"
4. 校验通过后，dispatch `NodeCallback(Success)`

### 11.5 状态机

```
execute()
  → node: Suspended, instance: Suspended
  → output = { mode, wait_seconds, resume_at }

[Auto] Sweeper 检测到 resume_at 过期
  → dispatch NodeCallback(Success)
  → handle_callback (默认实现) → node: Success
  → 工作流继续

[Manual] 用户调用 resume-node API
  → 服务端校验 resume_at <= now
  → dispatch NodeCallback(Success)
  → handle_callback (默认实现) → node: Success
  → 工作流继续
```

`handle_callback` 使用 `PluginInterface` trait 的默认实现，无需自定义——回调 Success 直接映射为节点 Success。

### 11.6 前端展示

| 场景 | UI |
|------|-----|
| **编辑器** | 属性面板显示"等待时间（秒）"和"模式"下拉选择 |
| **实例详情 — Auto** | 节点详情显示倒计时和模式；Sweeper 自动推进后刷新 |
| **实例详情 — Manual** | 显示倒计时；到期后出现"确认继续"按钮 |
| **通用** | 暂停节点同样支持"跳过"操作（复用现有 skip 逻辑） |

---

## 12. 工作流模板持久化 (WorkflowEntity)

### 12.1 保存与 upsert 行为

`save_workflow_entity` 使用 MongoDB **upsert**：按 `(workflow_meta_id, version)` 查找，若文档已存在则更新，否则插入。

- **仅 `Draft` 状态的版本可被更新**。对已存在且状态为 `Published` 或 `Archived` 的版本发起保存（更新）时，API 返回 **422**（Unprocessable Entity）。

### 12.2 版本生命周期与 WorkflowStatus

`WorkflowEntity` 的版本状态由 `WorkflowStatus` 枚举表示，生命周期如下：

```
Draft → (可反复更新 upsert) → Published → Archived → Deleted(标记删除)
```

| 状态 | 说明 |
|------|------|
| **Draft** | 可编辑、可保存 |
| **Published** | 不可修改，可发起实例 |
| **Archived** | 不可修改，已运行实例可继续，不可新发起 |
| **Deleted** | 标记删除 |

### 12.3 唯一索引

MongoDB 在应用启动时通过 `ensure_indexes()` 确保以下唯一索引存在：

| 集合 | 唯一索引字段 |
|------|----------------|
| `workflow_entities` | `(workflow_meta_id, version)` |
| `workflow_meta_entities` | `(workflow_meta_id)` |

### 12.4 发布接口

将指定 **Draft** 版本变更为 **Published**：

```
POST /api/v1/workflow/meta/{workflow_meta_id}/template/{version}/publish
```

---

## 13. 扫地僧 (Sweeper) — 僵尸实例自动恢复

### 13.1 问题场景

引擎崩溃、Redis 丢消息等故障发生后，部分工作流实例可能永远停在非终态而无人推进：

| 状态 | 成因 | 表现 |
|------|------|------|
| **Running** | Worker 持锁时引擎宕机，未释放锁也未推进 | 租约过期后锁可被抢占，但无人再投递事件 |
| **Await** | 子任务回调在 Redis 中丢失（如 Redis 重启、队列 TTL 淘汰） | 工作流永远等不到 NodeCallback |

### 13.2 恢复判定：仅基于 `locked_at` + `locked_duration`

扫地僧**不依赖** `updated_at`（该字段仅为业务记录，不具备分布式安全语义），也**不依赖** `locked_by`（仅用于展示当前持锁者，不参与并发安全判定）。判定依据**完全且仅来自 `locked_at` + `locked_duration`**：

```
扫描目标（MongoDB 查询）：
  status ∈ { Running, Await }
  AND (
    locked_at IS NULL                        -- 无租约（锁已释放或从未加锁）
    OR locked_at + locked_duration < now()   -- 租约已过期（持锁者可能已崩溃）
  )
```

两种条件的语义：
- **`locked_at IS NULL`**：`release_lock` 正常清空了 `locked_at`，但实例仍在 Running/Await → 释放锁后无人推进后续事件
- **`locked_at + locked_duration < now()`**：租约到期，持锁 Worker 可能已宕机

**核心原则**：如果 `locked_at` 非空且 `locked_at + locked_duration >= now()`（租约有效期内），无论实例状态如何，扫地僧**绝不触碰**——因为有 Worker 正在处理。

### 13.3 恢复流程

```
对每个匹配的僵尸实例：

Step 1: 抢占式清锁（CAS 保护）
  filter:  { workflow_instance_id, epoch: <读到的epoch> }
  update:  { locked_by: null, locked_at: null, locked_duration: null }
  $inc:    { epoch: +1 }
  ↳ epoch CAS 保证：若其他 Worker 在扫地僧读取后已处理该实例（epoch 变了），
    则 CAS 失败，扫地僧跳过（幂等安全）

Step 2: 分支处理

  ┌─ 实例 status == Running:
  │   状态转换: Running → Pending (CAS)
  │   投递: dispatch_workflow(Start)
  │   ↳ Worker 拿到 Start → execute_workflow_loop → 从 current_node 恢复
  │
  └─ 实例 status == Await:
     子任务回查（见 §13.4）
     ↳ 根据回查结果决定：补发 NodeCallback 或重投 TaskJob
```

### 13.4 Await 状态的子任务回查

对 Await 状态的实例，仅投递 Start **不够**（`execute_workflow_loop` 遇到 Await 节点会直接 Stop）。需要主动回查子任务的实际状态：

#### 12.4.1 Http 节点（单子任务）

```
查询: task_instances.find({ task_instance_id: "{instance_id}-{node_id}" })

├─ task_status == Completed / Failed:
│   回调丢失 → 补发 NodeCallback { status, output, error_message }
│
├─ task_status == Pending / Running:
│   子任务也可能僵死 → 重投 dispatch_task(ExecuteTaskJob)
│
└─ 不存在:
│   ensure_task_instance_for_job 可能未完成 → 走 Running 恢复流程
```

#### 12.4.2 Parallel / ForkJoin 节点（多子任务）

```
从 node.task_instance.output 读取状态机:
  { total_items, dispatched_count, success_count, failed_count }

已知完成数 = success_count + failed_count
预期完成数 = dispatched_count（已派发的子任务数）

差值 = dispatched_count - 已知完成数  // 丢失的回调数

对每个丢失的回调:
  查询: task_instances.find({ task_instance_id: "{instance_id}-{node_id}-{index}" })
  ├─ Completed / Failed → 补发 NodeCallback
  └─ Pending / Running → 重投 dispatch_task
```

### 13.5 幂等安全性分析

| 场景 | 是否安全 | 原因 |
|------|:---:|------|
| 实例正在被 Worker 处理 | ✅ | 租约未过期 → 不匹配扫描条件 |
| 扫地僧与正常回调同时到达 | ✅ | Step 1 的 epoch CAS 保证只有一个成功 |
| Start 被重复投递 | ✅ | `execute_workflow_loop` 每次 reload from DB，幂等 |
| NodeCallback 被重复补发 | ✅ | `handle_callback` 应幂等（Parallel 的 success_count 已在库中，重复+1 需额外保护） |
| 子任务 TaskJob 被重复投递 | ✅ | TaskExecutor 执行是幂等的（HTTP 请求本身可能不幂等，但这是业务层面） |

**注意**：Parallel 的 `handle_callback` 中 `success_count` 的增量操作需要确保幂等（如通过 child_task_id 去重），否则补发的回调可能导致计数错误。建议在 Phase 2 实现时增加回调去重机制。

### 13.6 配置参数

```toml
[sweeper]
enabled = true
interval_secs = 60           # 扫描间隔
max_recover_per_cycle = 10   # 每次扫描最多恢复的实例数（限流防雪崩）
```

**不设置独立的超时阈值**——过期判定完全由每个实例自身的 `locked_at + locked_duration` 决定。这避免了引入额外的"魔法数字"，也意味着 `acquire_lock(duration_ms)` 的参数设定直接决定了该实例的最大无响应容忍时间。

### 13.7 可观测性

```rust
// 每次扫描循环记录
info!(
    scanned = scanned_count,          // 扫描到的僵尸实例数
    recovered_running = running_count, // Running → 重投 Start
    recovered_await = await_count,     // Await → 子任务回查 + 补发
    skipped_cas = cas_fail_count,      // CAS 失败（已被其他 Worker 处理）
    "sweeper cycle completed"
);

// 每个恢复的实例单独记录
info!(
    workflow_instance_id = %id,
    original_status = %status,
    action = %action,  // "restarted" | "callback_补发" | "task_redispatch"
    "sweeper recovered instance"
);
```

### 13.8 实现分阶段

| 阶段 | 内容 | 覆盖场景 |
|------|------|----------|
| **Phase 1 (MVP)** | 扫描 Running + 锁过期 → 清锁 + Running→Pending + 投递 Start | 引擎宕机后 Running 实例恢复 |
| **Phase 2** | 扫描 Await + 锁过期 → 子任务状态回查 + 补发 NodeCallback / 重投 TaskJob | Redis 丢消息后 Await 实例恢复 |
| **Phase 3** | Parallel/ForkJoin 回调去重（child_task_id based）+ 审批超时扫描 + metrics 埋点 | 完整生产可用 |

### 13.10 回调去重机制（Phase 3）

Sweeper 补发回调与原始回调可能并发到达，必须在 Parallel / ForkJoin 的 `handle_callback` 中做幂等判断。

**实现方式**：在 `node_instance.task_instance.output` 的 state JSON 中维护 `processed_callbacks: string[]` 数组和 `results: Map<child_id, {status, output, error}>` map。

```
callback 到达
  → 读取 state.processed_callbacks
  → child_task_id ∈ processed_callbacks ?
      ├── YES & status ≠ Skipped → warn!("duplicate callback ignored") → 返回空调度（幂等跳过）
      ├── YES & status = Skipped → **覆盖模式**：
      │     从 results[child_task_id] 读取原始状态
      │     回退原始计数器（如 Failed → failed_count -= 1）
      │     计入新状态（success_count += 1, skipped_count += 1）
      │     更新 results[child_task_id] 为 Skipped
      │     重新评估完成条件
      └── NO  → 正常累加 success_count / failed_count
               → 记录 results[child_task_id] = {status, output, error}
               → 将 child_task_id 追加到 processed_callbacks
               → 持久化 state
```

> **设计决策**：Skipped 是用户主动发起的操作，必须能覆盖之前的 Failed 结果。若不允许覆盖，容器会因所有 child 都在 `processed_callbacks` 中而永远卡在 Await 状态（死锁）。非 Skipped 的重复回调仍然被忽略，保证 Sweeper 补发和并发回调的幂等性。

**适用插件**：
- `ParallelPlugin::handle_callback` — `child_task_id` 格式: `{wf_id}-{node_id}-{index}`
- `ForkJoinPlugin::handle_callback` — 同上格式

### 13.11 审批超时自动驳回（Phase 3）

Sweeper 在每个扫描周期还会处理过期审批：

```
scan_expired_approvals(limit)
  → 查询 status="Pending" AND expires_at < now AND expires_at IS NOT NULL
  → 对每个过期审批:
      1. 标记 status=Rejected, updated_at=now
      2. 投递 NodeCallback(Failed) 到工作流
         output: { approval_expired: true, expires_at }
         error_message: "approval expired"
```

`ApprovalService` 新增 `scan_expired_approvals` + `expire_approval` 方法，Sweeper 通过 `with_approval_service()` 可选注入。

### 13.12 暂停节点到期自动唤醒

Sweeper 在每个扫描周期还会处理到期的 Auto 暂停节点：

```
sweep_expired_pause_nodes():
  → 查询 status == Suspended 的工作流实例
  → 遍历实例节点，查找同时满足以下条件的节点:
      node_type == Pause
      AND node.status == Suspended
      AND task_template.mode == Auto
      AND output.resume_at <= now()
  → 对每个匹配节点:
      dispatch NodeCallback(Success) 到工作流
```

此逻辑复用 Sweeper 已有的 `supplement_callback` 方法，与审批超时驳回共享回调投递通道。Manual 模式的暂停节点不由 Sweeper 处理，需用户通过 `resume-node` API 主动确认（详见 §11.4）。

### 13.9 架构位置

扫地僧运行在 engine 进程中，作为独立的 `tokio::spawn` 定时任务。**不经过** Apalis 队列（它自己就是队列的补偿机制），直接读写 MongoDB 并向 Redis 投递恢复 Job。

```
engine 进程
├── Workflow Worker (apalis consumer)
├── Task Worker (apalis consumer)
└── Sweeper (tokio interval task)  ← 新增
    ├── 读 MongoDB: 查询僵尸实例
    ├── 写 MongoDB: CAS 清锁 + 状态转换
    └── 写 Redis: 投递 Start / NodeCallback / TaskJob
```

---

## 14. API Key 系统

### 14.1 设计总则

API Key 为外部系统（CI/CD、脚本、第三方集成）提供程序化 API 访问能力。采用 **独立实体 + Token Exchange** 模式：API Key 是独立于 `UserEntity` 的实体，不侵入现有账号体系；API Key 不直接用于 API 调用，而是通过 Token Exchange 端点换取短效 JWT，再用 JWT 访问业务 API。

#### 设计选型

| 决策 | 选项 | 结论 |
|------|------|------|
| 实体模型 | A. 独立实体 / B. 虚拟用户 | **A. 独立实体**——零侵入现有 `users`/`user_tenant_roles`，职责清晰（人 = User，机器 = ApiKey） |
| 哈希算法 | SHA-256 / bcrypt | **SHA-256**——API Key 不像密码（泄露后直接吊销），查询性能优先 |
| 认证流 | 直接认证（每次查 DB）/ Token Exchange | **Token Exchange**——auth_middleware 无需改动，JWT 本地验证零 DB 开销，API Key 减少网络暴露 |
| API Key 角色 | 允许 TenantAdmin / 禁止 | **禁止 TenantAdmin**——防止通过 API Key 间接提权 |

### 14.2 数据模型

```rust
pub enum ApiKeyStatus {
    Active,
    Revoked,
}

pub struct ApiKeyEntity {
    pub id: String,                          // UUID 主键
    pub tenant_id: String,                   // 所属租户
    pub name: String,                        // 可读名称（如 "CI/CD Pipeline"）
    pub key_prefix: String,                  // key 前 8 位明文（用于展示和索引，如 "wfk_a3b1"）
    pub key_hash: String,                    // SHA-256(完整 key)
    pub role: TenantRole,                    // Developer / Operator / Viewer（禁止 TenantAdmin）
    pub expires_at: Option<DateTime<Utc>>,   // API Key 自身有效期，None = 永不过期
    pub token_ttl_secs: u32,                 // 签发的 JWT 有效期（秒），默认 3600，上限 86400
    pub last_used_at: Option<DateTime<Utc>>, // 最后一次 Token Exchange 时间
    pub status: ApiKeyStatus,                // Active / Revoked
    pub created_by: String,                  // 创建者 user_id
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 14.3 Key 格式与存储

```
格式: wfk_{32字符Base62随机串}
示例: wfk_a3b1c4d5e6f7g8h9i0j1k2l3m4n5o6p7
```

| 字段 | 存储内容 | 用途 |
|------|----------|------|
| `key_prefix` | `"wfk_a3b1"` (前 8 字符) | 唯一索引，用于快速查找和列表展示 |
| `key_hash` | `SHA-256(完整 key)` | 验证 key 正确性 |

**完整明文 key 仅在创建时返回一次**，此后不可再获取。

### 14.4 认证流程（Token Exchange）

```
外部系统 使用 API Key:

Phase 1 — 换取短效 JWT（低频，仅在 JWT 过期时）:
  POST /api/v1/api-keys/token
  Body: { "key": "wfk_a3b1c4d5e6f7g8h9i0j1k2l3m4n5o6p7" }

  服务端处理:
    1. 截取 key_prefix = "wfk_a3b1"
    2. 查 api_keys 集合（key_prefix 唯一索引）
    3. SHA-256(请求中的 key) == 记录.key_hash ?
    4. 校验 status == Active
    5. 校验 expires_at IS NULL || expires_at > now()
    6. 更新 last_used_at = now()
    7. 签发 JWT（Claims: sub="apikey:{id}", username=name,
       is_super_admin=false, tenant_id, role, exp=now+token_ttl_secs）

  响应:
    {
      "access_token": "eyJhbGciOiJIUzI1NiI...",
      "token_type": "Bearer",
      "expires_in": 3600
    }

Phase 2 — 用 JWT 调用业务 API（高频，纯内存验证）:
  Authorization: Bearer eyJhbGciOiJIUzI1NiI...
  → 现有 auth_middleware → verify_token → AuthContext
  → 业务 Handler
```

**关键优势**：auth_middleware **无需任何改动**——Token Exchange 签发的是标准 JWT，与人类用户登录后的 JWT 格式完全一致。区别仅在 `Claims.sub` 的值前缀为 `apikey:`，用于审计追溯。

### 14.5 Token Exchange 端点

```
POST /api/v1/api-keys/token  (公开端点，无需 JWT 认证)

请求:
  { "key": "wfk_xxxxxxxxxxxx" }

成功响应 (200):
  {
    "access_token": "eyJhbG...",
    "token_type": "Bearer",
    "expires_in": 3600
  }

失败响应:
  401 — key 无效、已过期、已吊销
```

### 14.6 JWT 有效期策略

| 参数 | 默认值 | 上限 | 说明 |
|------|--------|------|------|
| `token_ttl_secs` | 3600 (1小时) | 86400 (24小时) | 创建 API Key 时指定 |

**吊销生效延迟**：API Key 被吊销后，已签发的 JWT 在到期前仍有效（最长 = `token_ttl_secs`）。新的 Token Exchange 请求会被立即拒绝。

对于 MVP，**不实现 JWT 黑名单**。若后续需要即时吊销，可扩展为：

1. JWT Claims 增加 `jti`（JWT ID）
2. 吊销时将 `jti` 写入 Redis 黑名单（TTL = 剩余有效期）
3. auth_middleware 增加 Redis 黑名单检查

### 14.7 API 路由

```
# API Key 管理（受保护，需要 UserManage 权限 = TenantAdmin+）
POST   /api/v1/api-keys                 # 创建 → 返回一次性 key 明文
GET    /api/v1/api-keys                 # 列表（仅 prefix + name + role + 状态，不含 hash）
DELETE /api/v1/api-keys/{id}            # 吊销 → status = Revoked

# Token Exchange（公开端点，挂在 public_routes，与 /auth/login 同级）
POST   /api/v1/api-keys/token           # API Key → 短效 JWT
```

### 14.8 管理 API 详情

#### 创建 API Key

```
POST /api/v1/api-keys
Body: {
  "name": "CI Pipeline",
  "role": "Developer",            // Developer | Operator | Viewer（禁止 TenantAdmin）
  "expires_at": "2027-01-01T00:00:00Z",  // 可选，null = 永不过期
  "token_ttl_secs": 3600          // 可选，签发 JWT 有效期，默认 3600
}

Response (201):
{
  "id": "uuid",
  "name": "CI Pipeline",
  "key": "wfk_a3b1c4d5e6f7g8h9i0j1k2l3m4n5o6p7",  // 仅此一次返回明文
  "key_prefix": "wfk_a3b1",
  "role": "Developer",
  "expires_at": "2027-01-01T00:00:00Z",
  "token_ttl_secs": 3600,
  "created_at": "..."
}
```

#### 列表

```
GET /api/v1/api-keys

Response:
[
  {
    "id": "uuid",
    "name": "CI Pipeline",
    "key_prefix": "wfk_a3b1",
    "role": "Developer",
    "expires_at": "2027-01-01T00:00:00Z",
    "token_ttl_secs": 3600,
    "last_used_at": "2026-04-10T12:00:00Z",
    "status": "Active",
    "created_by": "user_uuid",
    "created_at": "..."
  }
]
```

#### 吊销

```
DELETE /api/v1/api-keys/{id}

Response (200): { "data": null }
```

### 14.9 角色约束

| 可创建的 API Key 角色 | 说明 |
|----------------------|------|
| Developer | 模板读写 + 实例执行 |
| Operator | 仅实例执行 |
| Viewer | 只读 |

**TenantAdmin 角色禁止通过 API Key 授予**——防止机器凭证获得用户管理权限。SuperAdmin 不可通过 API Key 模拟。

### 14.10 审计追溯

通过 API Key 签发的 JWT，其 `Claims.sub` 格式为 `apikey:{api_key_id}`。所有后续 API 调用中 `AuthContext.user_id` 均携带此前缀，可在日志和审计系统中区分人工操作与机器操作。

### 14.11 MongoDB 集合

```
集合名: api_keys

唯一索引:
  - (key_prefix)       — 用于 Token Exchange 快速查找
  - (tenant_id, name)  — 同租户内名称唯一

文档结构:
{
  id: String,
  tenant_id: String,
  name: String,
  key_prefix: String,
  key_hash: String,
  role: TenantRole,
  expires_at: Option<DateTime>,
  token_ttl_secs: u32,
  last_used_at: Option<DateTime>,
  status: ApiKeyStatus,
  created_by: String,
  created_at: DateTime,
  updated_at: DateTime
}
```

### 14.12 前端页面

在用户管理或租户设置区域新增 "API Keys" 管理页面：

| 功能 | UI |
|------|-----|
| **列表** | 表格展示 name、key_prefix（掩码 `wfk_a3b1****`）、角色、有效期、最后使用时间、状态 |
| **创建** | 表单：名称、角色下拉（Developer/Operator/Viewer）、有效期日期选择器、JWT TTL 输入 |
| **创建成功** | 弹窗显示完整 key + 复制按钮 + 警告"关闭后不可再查看" |
| **吊销** | 确认弹窗后标记 Revoked |

### 14.13 安全考量

| 维度 | 策略 |
|------|------|
| Key 存储 | SHA-256 哈希，不存明文 |
| 网络暴露 | API Key 仅在 Token Exchange 时传输（低频），JWT 用于高频调用 |
| 权限上限 | API Key 不可拥有 TenantAdmin / SuperAdmin 权限 |
| 有效期 | API Key 可设自定义过期时间；JWT 有效期独立控制（上限 24h） |
| 吊销 | 立即阻止新 JWT 签发；已签发 JWT 在到期前仍有效（MVP 不做黑名单） |
| Rate Limit | 后续可对 Token Exchange 端点做限速，防止暴力破解 |

### 14.14 实施步骤

| 步骤 | 内容 |
|------|------|
| 1 | `domain/apikey/entity` — ApiKeyEntity + ApiKeyStatus |
| 2 | `domain/apikey/repository` — ApiKeyRepository trait |
| 3 | `infrastructure/mongodb/apikey` — MongoDB 实现 + 唯一索引 |
| 4 | `domain/apikey/service` — 创建（生成 key + SHA-256）、认证+签发 JWT、吊销、列表 |
| 5 | `api/handler/apikey` — 管理 API（CRUD）+ Token Exchange 公开端点 |
| 6 | `api/router` — 管理路由挂 protected（UserManage），Token Exchange 挂 public |
| 7 | 前端：API Key 管理页面 |

---

## 15. LLM 大模型插件 (LLM Plugin)

### 15.1 设计总则

LLM 插件为工作流提供大语言模型调用能力，遵循与 HTTP 任务完全对齐的架构模式：**Prompt 模板预设 + 运行时变量插值 + 异步 Task Worker 执行**。

#### 边界定义

**边界内**（task_llm 该做的）：
- 预设好的 Prompt 模板 + 运行时 `{{变量}}` 插值
- 调用 LLM API 获取结果
- 结果结构化写入 output，供下游节点使用
- 重试、超时、错误处理
- 独立可调用（TaskEntity）+ 工作流可编排（内嵌节点）

**边界外**（task_llm 不该做的）：
- 不是对话机器人（没有多轮会话管理）
- 不是 Agent 框架（不自主决策下一步行动）
- 不是 Prompt 工程平台（不做 A/B 测试、Prompt 版本管理）
- 不自己管理 API Key（复用已有的 Variable 加密存储）

#### 关键设计决策

| 决策 | 选项 | 结论 |
|------|------|------|
| Provider 抽象 | A. 统一 OpenAI 兼容 / B. 多适配器 / C. 仅 OpenAI + 退回 HTTP | **A. 统一 OpenAI 兼容接口**——覆盖 OpenAI/Azure/Ollama/vLLM/LiteLLM/各大云厂商兼容 API |
| API Key 管理 | A. 模板明文 / B. 引用变量 / C. 新实体 | **B. 引用 Variable 系统**——复用 §8 变量系统的 Secret 加密存储，零新增实体 |
| 输出处理 | A. 原始返回 / B. 结构化提取 / C. JSON Mode | **B+C 结合**——content 提取 + usage 元信息 + 可选 JSON Mode 自动 parse |
| 流式 vs 非流式 | A. 非流式 / B. 流式 | **A. 仅非流式**——工作流需要完整结果，流式对编排无价值 |
| 多轮对话 | A. 单次调用 / B. 从 context 读历史 | **A. V1 仅单次调用**——system_prompt + user_prompt 覆盖主要场景 |
| Function Calling | A. 不支持 / B. 支持 | **A. V1 不支持**——编排逻辑应在工作流层面，不交给 LLM 决策 |

### 15.2 模板定义

```rust
pub enum LlmResponseFormat {
    Text,       // 默认，不约束输出格式
    JsonObject, // 约束 LLM 输出为 JSON 对象
}

pub struct LlmTemplate {
    pub base_url: String,                         // OpenAI 兼容 API 地址，如 "https://api.openai.com/v1"
    pub model: String,                            // 模型名称，如 "gpt-4o", "claude-sonnet-4-20250514"
    pub api_key_ref: String,                      // 引用 Variable 的 key（Secret 类型，加密存储）
    pub system_prompt: Option<String>,            // 系统提示词，支持 {{变量}} 模板插值
    pub user_prompt: String,                      // 用户提示词，支持 {{变量}} 模板插值
    pub temperature: Option<f64>,                 // 温度 0.0-2.0，None 使用模型默认值
    pub max_tokens: Option<u32>,                  // 最大输出 token 数，None 使用模型默认值
    pub timeout: u32,                             // 超时秒数
    pub retry_count: u32,                         // 重试次数（0 = 不重试）
    pub retry_delay: u32,                         // 重试间隔秒数
    pub response_format: Option<LlmResponseFormat>, // 输出格式约束
}
```

**与 HTTP 模板的类比**：

| HTTP 模板字段 | LLM 模板字段 | 说明 |
|--------------|-------------|------|
| `url` | `base_url` + `model` | 请求目标 |
| `headers` (API Key) | `api_key_ref` → Variable | 认证凭证 |
| `body` | `system_prompt` + `user_prompt` | 请求内容 |
| `retry_count` / `retry_delay` / `timeout` | 同名 | 重试与超时策略 |
| `success_condition` | `response_format` | 输出校验 |

### 15.3 Prompt 插值机制

Prompt 支持 `{{变量}}` 模板插值，与 HTTP 任务的 `Variable` 类型表单行使用同一套解析机制（`{{点路径}}` 占位符替换）。解析上下文来自 `resolve_variables` + `nodes` 注入后的合并结果（见 §8.5-8.6）。

**示例**：

模板定义：
```
system_prompt: "你是一个文本分类专家，可选类别为：{{categories}}"
user_prompt: "请将以下文本分类到上述类别中：\n\n{{input_text}}\n\n返回 JSON 格式：{\"category\": \"...\", \"confidence\": 0.0}"
```

运行时 context：
```json
{
  "categories": "科技, 金融, 医疗, 教育",
  "input_text": "OpenAI 发布了新版 GPT 模型..."
}
```

渲染后的实际请求：
```
system_prompt: "你是一个文本分类专家，可选类别为：科技, 金融, 医疗, 教育"
user_prompt: "请将以下文本分类到上述类别中：\n\nOpenAI 发布了新版 GPT 模型...\n\n返回 JSON 格式：{\"category\": \"...\", \"confidence\": 0.0}"
```

**引用前序节点输出**：

```
user_prompt: "基于以下 HTTP 响应内容进行摘要：\n\n{{nodes.http_1.output.body}}"
```

### 15.4 架构集成

LLM 插件完全遵循 §1.2 / §2 的双层架构（编排插件 + 执行器）：

```
┌─ Workflow Worker (编排侧) ─────────────────────────────────────────┐
│  LlmPlugin implements PluginInterface                              │
│    execute():                                                      │
│      1. 解析 LlmTemplate                                          │
│      2. 构造 ExecuteTaskJob (包含 caller_context)                   │
│      3. 返回 ExecutionResult::async_dispatch(job)                  │
│    handle_callback():                                              │
│      → 默认实现（拷贝结果，映射状态）                                 │
└────────────────────────────────────────────────────────────────────┘
         │ dispatch via Redis Queue
         ▼
┌─ Task Worker (执行侧) ────────────────────────────────────────────┐
│  LlmTaskExecutor implements TaskExecutor                           │
│    execute_task():                                                 │
│      1. 从 TaskInstanceEntity 提取 LlmTemplate                    │
│      2. 从 task_instance.input 获取渲染后的 prompts                │
│      3. 从 task_instance.input 获取解密后的 API Key                │
│      4. 构造 OpenAI Chat Completions 请求                          │
│      5. 发送 HTTP 请求（重试 + 超时）                               │
│      6. 解析响应，结构化输出                                        │
│      7. 返回 TaskExecutionResult                                   │
└────────────────────────────────────────────────────────────────────┘
```

### 15.5 执行时序

```mermaid
sequenceDiagram
    participant WW as Workflow Worker
    participant RQ as Redis Queue
    participant TW as Task Worker
    participant LLM as LLM API (OpenAI 兼容)
    participant DB as MongoDB

    Note over WW, DB: 1. 工作流命中 LLM 节点
    WW->>WW: resolve_variables() + 插值 Prompts
    WW->>WW: LlmPlugin::execute() → async_dispatch(job)
    WW->>RQ: 投递 ExecuteTaskJob
    WW->>DB: 实例状态 → Await

    Note over RQ, LLM: 2. Task Worker 执行 LLM 调用
    RQ->>TW: 消费 ExecuteTaskJob
    TW->>DB: 获取 TaskInstanceEntity (含渲染后的 prompts)
    TW->>LLM: POST /chat/completions
    LLM-->>TW: 响应 (content + usage)
    TW->>DB: 更新 TaskInstanceEntity (input/output)

    Note over TW, WW: 3. 回调
    TW->>RQ: 投递 NodeCallback(Success/Failed)
    RQ->>WW: 唤醒 Workflow Worker
    WW->>WW: handle_callback → 拷贝结果
    WW->>DB: 节点 Success/Failed，继续下一节点
```

### 15.6 输出结构

LLM 执行器的 `output` 字段采用结构化格式：

```json
{
  "content": "这篇文本属于科技类别...",
  "usage": {
    "prompt_tokens": 150,
    "completion_tokens": 80,
    "total_tokens": 230
  },
  "model": "gpt-4o",
  "finish_reason": "stop",
  "parsed": {
    "category": "科技",
    "confidence": 0.95
  }
}
```

| 字段 | 说明 |
|------|------|
| `content` | LLM 回复的原始文本内容 |
| `usage` | Token 消耗统计 |
| `model` | 实际使用的模型（可能与请求模型不同，如别名重定向） |
| `finish_reason` | 停止原因（`stop` / `length` / `content_filter`） |
| `parsed` | 仅当 `response_format = JsonObject` 时存在，自动 parse content 为 JSON 对象 |

下游节点引用：
- `{{nodes.llm_1.output.content}}` — 获取回复文本
- `{{nodes.llm_1.output.parsed.category}}` — 获取结构化字段
- `{{nodes.llm_1.output.usage.total_tokens}}` — 获取 token 消耗

### 15.7 `input` 快照

与 HTTP 任务一致（§3.3），LLM 任务的 `task_instance.input` 记录渲染后的完整请求快照：

```json
{
  "base_url": "https://api.openai.com/v1",
  "model": "gpt-4o",
  "system_prompt": "你是一个文本分类专家，可选类别为：科技, 金融, 医疗, 教育",
  "user_prompt": "请将以下文本分类到上述类别中：\n\nOpenAI 发布了...",
  "temperature": 0.3,
  "max_tokens": 500,
  "response_format": "JsonObject"
}
```

**注意**：`api_key_ref` 指向的 Secret 变量解密后用于构造请求，但**不写入 input 快照**（与 §8.4 Secret 变量脱敏策略一致）。input 中仅记录 `"api_key_ref": "OPENAI_API_KEY"` 变量名，不记录值。

### 15.8 API Key 凭证管理

LLM 的 API Key 不引入新实体，而是复用 §8 变量系统的 **Secret 类型变量**：

```
用户操作流程:
  1. 在租户变量（或工作流模板变量）中创建 Secret 变量:
     key = "OPENAI_API_KEY", type = Secret, value = "sk-proj-xxxx..."
  2. 在 LLM 模板中引用: api_key_ref = "OPENAI_API_KEY"
  3. 运行时: resolve_variables() 解密 → 注入执行上下文 → LlmTaskExecutor 读取
```

**优势**：
- 复用已有加密存储（AES-256-GCM）和权限控制（Secret 变量仅 TenantAdmin+ 可创建/修改值）
- 租户变量 → 全租户所有 LLM 节点共享同一 API Key
- 模板变量 → 不同工作流可用不同 API Key
- 变量系统已支持 API 掩码返回，前端安全

### 15.9 OpenAI Chat Completions 请求规范

LlmTaskExecutor 向 `{base_url}/chat/completions` 发送标准 OpenAI 格式请求：

```json
{
  "model": "gpt-4o",
  "messages": [
    { "role": "system", "content": "<渲染后的 system_prompt>" },
    { "role": "user", "content": "<渲染后的 user_prompt>" }
  ],
  "temperature": 0.3,
  "max_tokens": 500,
  "response_format": { "type": "json_object" }
}
```

| 字段映射 | LlmTemplate → OpenAI API |
|----------|--------------------------|
| `system_prompt` | `messages[0].role="system"` |
| `user_prompt` | `messages[1].role="user"` |
| `temperature` | `temperature`（省略则不传，用模型默认） |
| `max_tokens` | `max_tokens`（省略则不传） |
| `response_format = JsonObject` | `response_format: { "type": "json_object" }` |
| `response_format = Text / None` | 不传 `response_format` 字段 |

**兼容性说明**：绝大多数 OpenAI 兼容 API（Ollama、vLLM、Azure OpenAI、各大云厂商）均支持上述标准字段。非兼容的 Provider（如原生 Anthropic API）建议通过 LiteLLM 等代理服务转换为兼容接口。

### 15.10 错误处理与重试

```
LlmTaskExecutor::execute_task()
  for attempt in 0..=(retry_count):
    构造 HTTP 请求 → POST {base_url}/chat/completions
    ├─ 成功 (2xx):
    │   ├─ response_format=JsonObject 且 content 不是合法 JSON:
    │   │   → 视为失败，重试（LLM 未遵守格式约束）
    │   └─ 正常 → return Success { content, usage, model, ... }
    ├─ 4xx (客户端错误):
    │   ├─ 429 (Rate Limited) → 重试（尊重 Retry-After header）
    │   └─ 其他 4xx → 立即失败，不重试（参数错误不会因重试修复）
    ├─ 5xx (服务端错误) → 重试
    └─ 超时/网络错误 → 重试

    if attempt < retry_count:
      sleep(retry_delay) // 如遇 429，取 max(retry_delay, Retry-After)

  全部重试耗尽 → return Failed { error_message }
```

### 15.11 与 Parallel/ForkJoin 的组合

LLM 任务可作为 Parallel/ForkJoin 的子任务，实现批量 LLM 处理：

**Parallel 场景**（数据并行）：对一批文本逐条调用 LLM 分类
```json
{
  "task_type": "Parallel",
  "task_template": {
    "items_path": "texts",
    "item_alias": "text",
    "task_template": {
      "Llm": {
        "base_url": "https://api.openai.com/v1",
        "model": "gpt-4o-mini",
        "api_key_ref": "OPENAI_API_KEY",
        "user_prompt": "分类：{{text}}",
        "timeout": 30,
        "retry_count": 2,
        "retry_delay": 3
      }
    },
    "concurrency": 5,
    "mode": "Rolling"
  }
}
```

**ForkJoin 场景**（任务并行）：同时调用不同模型做对比
```json
{
  "task_type": "ForkJoin",
  "task_template": {
    "tasks": [
      { "task_key": "gpt4", "task_template": { "Llm": { "model": "gpt-4o", "..." } } },
      { "task_key": "claude", "task_template": { "Llm": { "model": "claude-sonnet-4-20250514", "..." } } }
    ],
    "concurrency": 2,
    "mode": "Batch"
  }
}
```

### 15.12 前端编辑器

任务模板编辑器和工作流节点编辑器均需支持 LLM 配置：

| 区域 | 内容 |
|------|------|
| **基本配置** | base_url 输入（含默认值 `https://api.openai.com/v1`）、model 输入、api_key_ref 下拉（列出当前作用域的 Secret 变量） |
| **Prompt 编辑** | system_prompt 多行文本框（可选）、user_prompt 多行文本框（必填），支持 `{{}}` 变量提示 |
| **模型参数** | temperature 滑块（0.0-2.0）、max_tokens 数值输入 |
| **格式约束** | response_format 下拉：Text / JSON Object |
| **可靠性** | timeout 数值输入（秒）、retry_count 数值输入、retry_delay 数值输入（秒） |
| **实例详情** | 显示渲染后的 prompts（input）、LLM 响应内容（output.content）、token 消耗（output.usage）、parsed 结果 |

### 15.13 TaskType 与 TaskTemplate 扩展

```rust
// shared/workflow.rs
pub enum TaskType {
    Http,
    IfCondition,
    ContextRewrite,
    Parallel,
    ForkJoin,
    SubWorkflow,
    Grpc,
    Approval,
    Pause,
    Llm,   // 新增
}

// task/entity/task_definition.rs
pub enum TaskTemplate {
    Http(TaskHttpTemplate),
    Grpc,
    Approval(ApprovalTemplate),
    IfCondition(IfConditionTemplate),
    ContextRewrite(ContextRewriteTemplate),
    Parallel(ParallelTemplate),
    ForkJoin(ForkJoinTemplate),
    SubWorkflow(SubWorkflowTemplate),
    Pause(PauseTemplate),
    Llm(LlmTemplate),   // 新增
}
```

### 15.14 涉及代码变更

| 层级 | 变更 |
|------|------|
| `domain/shared/workflow.rs` | `TaskType` 新增 `Llm` |
| `domain/task/entity/task_definition.rs` | `TaskTemplate` 新增 `Llm(LlmTemplate)`；新增 `LlmTemplate`、`LlmResponseFormat` 结构体 |
| `domain/plugin/plugins/llm.rs` | 新增 `LlmPlugin`：`execute` 构造 `ExecuteTaskJob` + `async_dispatch`；`handle_callback` 使用默认实现 |
| `domain/plugin/plugins/mod.rs` | 新增 `pub mod llm;` |
| `domain/task/executors/llm.rs` | 新增 `LlmTaskExecutor`：Prompt 从 `input` 读取 → 构造 Chat Completions 请求 → reqwest 发送 → 解析响应 → 结构化 output |
| `domain/task/executors/mod.rs` | 新增 `pub mod llm;` |
| `bin/engine.rs` | 注册 `LlmPlugin` 到 `PluginManager`；注册 `LlmTaskExecutor` 到 `TaskManager` |
| `domain/plugin/manager/` | `run_node` 对 LLM 模板做 Prompt 插值（与 HTTP 的 `effective_http_request` 类似），将渲染后 prompts + api_key（解密但不落盘）写入 `task_instance.input` |
| 前端 `types/task.d.ts` | 新增 `LlmTemplate`、`LlmResponseFormat` 类型 |
| 前端 `utils/constants.ts` | `TASK_TYPE_MAP` 新增 `Llm` |
| 前端 `views/workflow/editor/` | 节点编辑器新增 LLM 配置面板 |
| 前端 `views/task/template/editor.vue` | 任务模板编辑器新增 LLM 选项 |

### 15.15 未来扩展（V2+）

| 特性 | 说明 | 优先级 |
|------|------|--------|
| 多轮对话 | `messages_from_context: Option<String>` 从 context 读取 messages 数组 | P1 |
| Function Calling | 定义 tools schema，LLM 返回 tool_call 后映射为子任务 | P2 |
| 流式输出 | SSE 流式传输中间结果（需前端 UI 配合） | P3 |
| 多 Provider 原生支持 | Anthropic Messages API、Google Gemini API 原生适配器 | P2 |
| Token 成本追踪 | 累计 usage 数据到租户维度，支持成本统计和配额管控 | P1 |
| Prompt 模板库 | 共享可复用的 Prompt 模板，版本管理 | P3 |

---

## 16. 通知系统 (Notification System)

### 16.1 设计原则

1. **异步解耦**：事件产生方只投递一条轻量事件到 apalis-redis 队列，零性能影响。独立的 Notification Worker 消费并分发。
2. **两层订阅**：全局（租户级）+ 资源级（工作流模板级），用户自主选择关注的事件类型和渠道。
3. **渠道可扩展**：V1 支持站内通知 + Webhook，架构上预留邮件 / IM 等渠道扩展点。
4. **容器节点不冒泡**：Parallel / ForkJoin / SubWorkflow 等容器只通知容器节点自身的状态转换，内部子任务事件不独立产生通知。
5. **Payload 精简**：通知内容简短但信息充足，附带页面跳转链接，详细信息由用户在页面查看。
6. **审批强制推送**：`approval.pending` 等审批类事件不走用户订阅，而是**直接推送**给审批人列表中的所有用户。审批通知和用户主动订阅是两个独立的业务领域。
7. **独立任务不通知**：V1 仅对工作流内的事件产生通知，独立执行的任务（无工作流上下文）不产生通知事件。

### 16.2 事件目录 (Event Catalog)

#### 16.2.1 用户可订阅的事件

| 事件类别 | 事件类型标识 | 描述 |
|---------|------------|------|
| **工作流实例** | `workflow.started` | 工作流实例开始执行 |
| | `workflow.completed` | 工作流实例成功完成 |
| | `workflow.failed` | 工作流实例执行失败 |
| | `workflow.canceled` | 工作流实例被取消 |
| | `workflow.suspended` | 工作流实例被挂起（审批/暂停） |
| | `workflow.resumed` | 工作流实例从挂起恢复 |
| **节点执行** | `node.started` | 节点开始执行 |
| | `node.success` | 节点执行成功 |
| | `node.failed` | 节点执行失败 |
| | `node.skipped` | 节点被跳过 |
| | `node.suspended` | 节点进入挂起（审批/暂停） |
| **暂停节点** | `pause.waiting` | 暂停节点进入等待 |
| | `pause.expired` | 暂停计时到期（手动模式待确认） |
| | `pause.resumed` | 暂停节点已恢复 |
| **系统** | `sweeper.recovered` | Sweeper 恢复了僵尸实例 |

#### 16.2.2 强制推送事件（不走订阅）

以下事件**直接推送**给相关用户，不依赖订阅配置：

| 事件类型标识 | 推送目标 | 描述 |
|------------|---------|------|
| `approval.pending` | 审批人列表 | 新审批待处理 |
| `approval.approved` | 审批发起方（工作流创建者） | 审批已通过 |
| `approval.rejected` | 审批发起方 | 审批已驳回 |
| `approval.expired` | 审批人列表 + 审批发起方 | 审批超时自动驳回 |

**设计理由**：审批是面向特定用户的主动通知行为（"你有一个审批需要处理"），与用户主动订阅事件（"我想关注某个工作流的状态"）是两个独立的业务领域。审批通知必须确保送达，不能因为用户没有配置订阅就遗漏。

**事件产生规则**：
- 工作流实例级事件：在 `WorkflowInstanceStatus` 变更时产生。
- 节点级事件：仅限工作流实例的**顶层节点**（`WorkflowNodeInstanceEntity` 直属节点），容器内部子任务不独立产生通知。
- 审批事件：在审批创建 / 审批状态变更 / Sweeper 到期处理时，强制推送。
- 暂停事件：在暂停节点创建 / Sweeper 到期处理时产生。
- 独立任务：**V1 不产生通知事件**。

### 16.3 订阅模型

两层订阅结构：

```
全局订阅（租户级）    →  用户选择事件类型 + 渠道
                        适用于该租户下所有工作流

资源级订阅（模板级）  →  用户选择事件类型 + 渠道 + 指定工作流模板 ID
                        覆盖全局设置（额外关注或屏蔽特定模板的事件）
```

#### 16.3.1 数据模型

```rust
/// 通知订阅
pub struct NotificationSubscription {
    pub subscription_id: String,
    pub tenant_id: String,
    pub user_id: String,

    /// 订阅范围
    pub scope: SubscriptionScope,
    /// 资源级时：资源类型，如 "workflow_meta"
    pub resource_type: Option<String>,
    /// 资源级时：资源 ID，如 workflow_meta_id
    pub resource_id: Option<String>,

    /// 订阅的事件类型列表（仅限 §16.2.1 中用户可订阅的事件）
    pub event_types: Vec<String>,

    /// 渠道配置
    pub channels: Vec<NotificationChannel>,

    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum SubscriptionScope {
    Global,
    Resource,
}

pub enum NotificationChannel {
    InApp,
    Webhook {
        url: String,
        /// 可选，用于 HMAC-SHA256 签名验证
        secret: Option<String>,
    },
}
```

#### 16.3.2 订阅匹配逻辑

当可订阅事件 `E(event_type, workflow_meta_id)` 产生时：
1. 查找租户下所有 `enabled=true` 的订阅。
2. **资源级优先**：如果用户对该 `workflow_meta_id` 有资源级订阅，仅使用资源级订阅的配置。
3. **全局兜底**：如果用户没有该模板的资源级订阅，使用全局订阅的配置。
4. 按匹配到的订阅的 `channels` 列表分别投递。

### 16.4 通知记录

```rust
/// 通知记录（站内通知 + Webhook 投递记录共用）
pub struct NotificationRecord {
    pub notification_id: String,
    pub tenant_id: String,
    pub user_id: String,

    pub event_type: String,
    /// 事件简要数据（不含 output/input 大字段）
    pub event_payload: serde_json::Value,

    /// 来源信息
    pub source_type: String,      // "workflow_instance" / "approval" / "sweeper"
    pub source_id: String,        // 实例 ID / 审批 ID 等
    pub workflow_meta_id: Option<String>,

    /// 页面跳转链接
    pub url: Option<String>,

    /// 各渠道投递状态
    pub channel_statuses: Vec<ChannelDeliveryStatus>,

    /// 站内通知已读状态
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

pub struct ChannelDeliveryStatus {
    pub channel: String,          // "in_app" / "webhook"
    pub status: DeliveryStatus,
    pub sent_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

pub enum DeliveryStatus {
    Pending,
    Sent,
    Failed,
}
```

**保留策略**：通知记录保留 **30 天**，过期由 MongoDB TTL 索引自动清理。

### 16.5 架构

#### 16.5.1 Dispatcher

通知系统使用**独立的 `NotificationDispatcher` trait**，与现有 `TaskDispatcher` 分离：

```rust
#[async_trait]
pub trait NotificationDispatcher: Send + Sync {
    async fn dispatch_notification(
        &self,
        event: NotificationEvent,
    ) -> anyhow::Result<()>;
}

pub struct NotificationEvent {
    pub tenant_id: String,
    pub event_type: String,
    pub workflow_meta_id: Option<String>,
    /// 强制推送的目标用户列表（审批事件使用）
    pub target_user_ids: Option<Vec<String>>,
    pub payload: serde_json::Value,
}
```

**`target_user_ids`**：
- 为 `None` 时：走订阅匹配逻辑（§16.3.2）。
- 为 `Some(users)` 时：**强制推送**给指定用户（审批事件），不查订阅，统一使用 InApp 渠道。

实现：`ApalisNotificationDispatcher`，将 `NotificationEvent` 投递到 apalis-redis 的 `notification_events` 队列。

#### 16.5.2 Worker

```
事件产生方 (Workflow Worker / Sweeper / API Handler)
       │
       │  notification_dispatcher.dispatch_notification(event)
       ▼
  apalis-redis 队列 (notification_events)
       │
       ▼
  NotificationWorker (Engine 进程内，独立消费)
       │
       ├── target_user_ids 不为空 → 强制推送模式
       │      └── 为每个用户创建 InApp NotificationRecord
       │
       └── target_user_ids 为空 → 订阅匹配模式
              ├── 1. 按 tenant_id + event_type + workflow_meta_id 查找匹配订阅
              ├── 2. 资源级优先，全局兜底
              ├── 3. 按渠道分发：
              │      ├── InApp → 写入 notification_records (MongoDB)
              │      └── Webhook → HTTP POST（失败重试 3 次）
              └── 4. 更新 channel_statuses
```

**NotificationWorker 运行在 Engine 进程**，与 Workflow Worker / Task Worker / Sweeper 同进程，共享 apalis-redis 连接。V1 足够轻量，无需独立进程。

#### 16.5.3 事件嵌入点

各模块持有 `Arc<dyn NotificationDispatcher>` 并在关键时刻投递事件：

| 位置 | 事件 | 方式 |
|------|------|------|
| **Workflow Worker** `run_node` 成功后 | `node.success` | dispatch_notification |
| **Workflow Worker** `run_node` 失败后 | `node.failed` | dispatch_notification |
| **Workflow Worker** `apply_exec_result` 工作流终态 | `workflow.completed` / `workflow.failed` | dispatch_notification |
| **Workflow Worker** 工作流开始执行 | `workflow.started` | dispatch_notification |
| **Approval Plugin** 创建审批 | `approval.pending` | dispatch_notification (target_user_ids = 审批人) |
| **Approval Plugin** / API 审批完成 | `approval.approved` / `approval.rejected` | dispatch_notification (target_user_ids = 发起方) |
| **Sweeper** `sweep_expired_approvals` | `approval.expired` | dispatch_notification (target_user_ids = 审批人+发起方) |
| **Sweeper** `sweep_expired_pause_nodes` | `pause.expired` | dispatch_notification |
| **API Handler** 取消实例 | `workflow.canceled` | dispatch_notification |
| **API Handler** 跳过节点 | `node.skipped` | dispatch_notification |

#### 16.5.4 页面链接生成

Webhook payload 和站内通知中包含 `url` 字段，指向前端实例详情页面。

**配置**：在应用配置文件中新增 `frontend_base_url` 字段：

```toml
[notification]
frontend_base_url = "https://your-domain.com"
```

**链接格式**：
- 工作流实例相关：`{frontend_base_url}/workflows/instances/{workflow_instance_id}`
- 审批相关：`{frontend_base_url}/workflows/instances/{workflow_instance_id}`（跳到实例页查看审批节点）

### 16.6 Webhook 规范

#### 16.6.1 请求格式

```
POST {user_webhook_url}
Content-Type: application/json
X-Webhook-Event: workflow.failed
X-Webhook-Timestamp: 2026-04-10T10:30:00Z
X-Webhook-Signature: sha256=<hex_hmac>   (仅当配置了 secret 时)

{
    "event_type": "workflow.failed",
    "timestamp": "2026-04-10T10:30:00Z",
    "tenant_id": "d409a474-...",
    "data": {
        "workflow_instance_id": "abc123",
        "workflow_meta_id": "def456",
        "workflow_name": "数据同步",
        "version": 3,
        "status": "Failed",
        "failed_node": "fetch_data",
        "error_message": "HTTP 节点执行超时"
    },
    "url": "https://your-domain.com/workflows/instances/abc123"
}
```

#### 16.6.2 签名验证

当用户配置了 `secret` 时，请求头附带 `X-Webhook-Signature`：

```
X-Webhook-Signature: sha256=HMAC_SHA256(raw_request_body, secret)
```

接收方验签伪代码：
```python
import hmac, hashlib
expected = hmac.new(secret.encode(), request.body, hashlib.sha256).hexdigest()
actual = request.headers["X-Webhook-Signature"].removeprefix("sha256=")
assert hmac.compare_digest(expected, actual)
```

用户不配置 secret 时，不附带签名头，正常 POST。

#### 16.6.3 重试策略

Webhook 投递失败（网络异常、非 2xx 响应）时重试：
- 最多 **3 次**，间隔 5s / 15s / 30s（指数退避）。
- 最终仍失败则标记 `channel_statuses[webhook].status = Failed`，记录错误信息。

#### 16.6.4 各事件 Payload 示例

**workflow.completed**：
```json
{
    "event_type": "workflow.completed",
    "timestamp": "2026-04-10T10:30:00Z",
    "tenant_id": "d409a474-...",
    "data": {
        "workflow_instance_id": "abc123",
        "workflow_meta_id": "def456",
        "workflow_name": "数据同步",
        "version": 3,
        "status": "Completed"
    },
    "url": "https://your-domain.com/workflows/instances/abc123"
}
```

**node.failed**：
```json
{
    "event_type": "node.failed",
    "timestamp": "2026-04-10T10:31:00Z",
    "tenant_id": "d409a474-...",
    "data": {
        "workflow_instance_id": "abc123",
        "workflow_meta_id": "def456",
        "workflow_name": "数据同步",
        "node_id": "fetch_data",
        "node_type": "Http",
        "status": "Failed",
        "error_message": "connection timeout"
    },
    "url": "https://your-domain.com/workflows/instances/abc123"
}
```

**approval.pending（强制推送）**：
```json
{
    "event_type": "approval.pending",
    "timestamp": "2026-04-10T10:32:00Z",
    "tenant_id": "d409a474-...",
    "data": {
        "workflow_instance_id": "abc123",
        "workflow_meta_id": "def456",
        "workflow_name": "数据同步",
        "node_id": "manager_approve",
        "approval_id": "appr_789",
        "approvers": ["alice", "bob"],
        "mode": "Any",
        "timeout_seconds": 3600
    },
    "url": "https://your-domain.com/workflows/instances/abc123"
}
```

### 16.7 站内通知

#### 16.7.1 API 接口

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/v1/notifications` | 登录用户 | 分页查询当前用户通知列表 |
| GET | `/api/v1/notifications/unread-count` | 登录用户 | 获取未读通知数量 |
| PUT | `/api/v1/notifications/{id}/read` | 登录用户 | 标记单条已读 |
| PUT | `/api/v1/notifications/read-all` | 登录用户 | 标记全部已读 |

#### 16.7.2 前端交互

1. **Header 铃铛**：右上角通知图标 + 未读数量角标，轮询间隔 **30 秒**。
2. **通知下拉面板**：点击铃铛展开最近通知列表，每条显示事件类型图标 + 摘要文本 + 时间。
3. **通知列表页**：完整的通知历史，支持按事件类型筛选、标记已读 / 全部已读。
4. **点击跳转**：每条通知关联一个目标页面链接（如工作流实例详情页）。

### 16.8 订阅管理

#### 16.8.1 API 接口

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/v1/subscriptions` | 登录用户 | 查询当前用户的订阅列表 |
| POST | `/api/v1/subscriptions` | 登录用户 | 创建订阅 |
| PUT | `/api/v1/subscriptions/{id}` | 登录用户 | 更新订阅 |
| DELETE | `/api/v1/subscriptions/{id}` | 登录用户 | 删除订阅 |

#### 16.8.2 前端交互

**订阅管理页**（可放在用户中心或独立菜单）：
1. **全局订阅卡片**：事件类型多选 + 渠道配置（站内通知开关 + Webhook URL/Secret）。
2. **资源级订阅列表**：选择工作流模板 → 事件类型多选 + 渠道配置。
3. 每条订阅支持启用 / 禁用开关。

### 16.9 MongoDB 集合与索引

**`notification_subscriptions` 集合**：

```javascript
// 唯一索引：同一用户同一范围同一资源只有一条订阅
db.notification_subscriptions.createIndex(
    { tenant_id: 1, user_id: 1, scope: 1, resource_type: 1, resource_id: 1 },
    { unique: true }
)
```

**`notification_records` 集合**：

```javascript
// 查询索引：按用户 + 租户查通知列表
db.notification_records.createIndex({ tenant_id: 1, user_id: 1, created_at: -1 })
// 未读计数索引
db.notification_records.createIndex({ tenant_id: 1, user_id: 1, read: 1 })
// TTL 索引：30 天自动过期
db.notification_records.createIndex({ created_at: 1 }, { expireAfterSeconds: 2592000 })
```

### 16.10 代码变更清单

**后端**：

| 文件 | 变更 |
|------|------|
| `src/crates/domain/src/notification/entity/mod.rs` | `NotificationSubscription`、`NotificationRecord`、`NotificationEvent`、`NotificationChannel`、枚举等实体 |
| `src/crates/domain/src/notification/repository/mod.rs` | `NotificationSubscriptionRepository`、`NotificationRecordRepository` trait |
| `src/crates/domain/src/notification/service.rs` | `NotificationService`（订阅 CRUD、通知查询、已读标记、未读计数） |
| `src/crates/domain/src/notification/dispatcher.rs` | `NotificationDispatcher` trait 定义 |
| `src/crates/domain/src/notification/mod.rs` | 模块声明 |
| `src/crates/domain/src/lib.rs` | `pub mod notification;` |
| `src/crates/infrastructure/src/mongodb/notification/` | MongoDB Repository 实现 + 索引创建 |
| `src/crates/infrastructure/src/queue/dispatcher.rs` | `ApalisNotificationDispatcher` 实现（投递到 apalis-redis） |
| `src/crates/api/src/handler/notification/mod.rs` | 通知 API Handler（列表、未读数、已读） |
| `src/crates/api/src/handler/subscription/mod.rs` | 订阅管理 API Handler（CRUD） |
| `src/crates/api/src/router.rs` | 注册通知和订阅路由 |
| `src/crates/domain/src/plugin/manager/workflow.rs` | `PluginManager` 持有 `NotificationDispatcher`，节点/工作流状态变更时投递事件 |
| `src/crates/domain/src/plugin/plugins/approval.rs` | 审批创建时投递 `approval.pending` 强制推送 |
| `src/crates/domain/src/sweeper/mod.rs` | Sweeper 持有 `NotificationDispatcher`，各处理函数追加通知投递 |
| `src/bin/apiserver.rs` | 初始化 `NotificationService`、`NotificationDispatcher`、注册 Handler |
| `src/bin/engine.rs` | 初始化 `NotificationDispatcher`、注册 `NotificationWorker`、传递给 `PluginManager` 和 `Sweeper` |
| `config.toml` (或等效) | 新增 `[notification]` 段：`frontend_base_url` |

**前端**：

| 文件 | 变更 |
|------|------|
| `frontend/src/types/notification.d.ts` | 通知 + 订阅类型定义 |
| `frontend/src/api/notification.ts` | 通知 API 客户端 |
| `frontend/src/api/subscription.ts` | 订阅 API 客户端 |
| `frontend/src/components/layout/header-bar.vue` | 铃铛图标 + 未读角标 + 通知下拉面板 |
| `frontend/src/views/notification/list.vue` | 通知列表页 |
| `frontend/src/views/subscription/list.vue` | 订阅管理页 |
| `frontend/src/router/index.ts` | 新增路由 |
| `frontend/src/components/layout/sidebar-menu.vue` | 新增菜单项 |

### 16.11 未来扩展

1. **Notification 节点类型**：允许用户在工作流中主动插入通知步骤，作为可编排节点。
2. **邮件渠道 (SMTP)**：`NotificationChannel::Email { smtp_config }`。
3. **IM 渠道**：直接对接企业微信 / 钉钉 / Slack API（非 Webhook）。
4. **WebSocket / SSE 实时推送**：替代前端轮询，实现真正的实时通知。
5. **通知模板**：支持自定义通知文本模板（Webhook Payload / 站内通知摘要），使用 `{{variable}}` 插值。
6. **通知静默期**：用户可设置免打扰时间段。
7. **通知聚合 (Digest)**：高频事件在指定时间窗口内聚合为一条摘要通知。
8. **独立任务通知**：为独立执行的任务（HTTP / LLM）增加 `task.completed` / `task.failed` 事件。

## 17. 总结与扩展

当前基于 **PluginFactory -> Queue -> Executor -> Callback Event -> Plugin.handle_callback** 的大闭环，使得这个 Rust 工作流引擎从玩具级别跃升至了企业级微服务架构。

**实例运维补充**：重试/取消/跳过节点与级联通知的完整约定见 **§1.4**（**方案 A**：与执行器共用 **`NodeCallback`** 形态进入编排；**`Start`** 用于 `Pending` 整实例重入）。

**僵尸实例恢复**：见 **§13 扫地僧 (Sweeper)**，基于 `locked_at + locked_duration` 租约过期判定 + epoch CAS 安全清锁，三阶段全部实现：Running 重投、Await 回调补发、Parallel/ForkJoin 回调去重、审批超时自动驳回。

**暂停节点**：见 **§11 暂停节点 (Pause Plugin)**，支持 Auto（Sweeper 驱动定时唤醒）和 Manual（人工确认 + 服务端时间校验）两种模式。

**API Key 系统**：见 **§14 API Key 系统**，采用独立实体 + Token Exchange 模式，零侵入现有账号体系，API Key 换取短效 JWT 用于 API 调用。

**LLM 大模型插件**：见 **§15 LLM Plugin**，采用统一 OpenAI 兼容接口，与 HTTP 任务完全对齐的双层架构（编排插件 + 执行器），复用 Variable 系统管理 API Key，支持 Prompt 模板插值和 JSON Mode 结构化输出。

**通知系统**：见 **§16 通知系统 (Notification System)**，异步队列驱动，两层订阅（全局 + 资源级），V1 支持站内通知 + Webhook（HMAC-SHA256 签名），30 天自动过期，容器节点不冒泡。

**审批增强**：见 **§10.8–10.10**。§10.8 为 `WorkflowInstanceEntity` 和 `ApprovalInstanceEntity` 增加发起人/申请人字段（`created_by` / `applicant_id`），子工作流继承根发起人，`__initiator__` 系统变量注入执行上下文。§10.9（规划）基于 `applicant_id` 实现自审批策略（默认跳过发起人）。§10.10（规划）增强跳过审批节点的权限控制和审计。

未来的扩展方案：
1. **Prometheus metrics 埋点**：sweeper_recovered_total / sweeper_expired_approvals_total / sweeper_expired_pauses_total 等 Counter。
2. **JWT 即时吊销**：通过 `jti` + Redis 黑名单实现 API Key 吊销后 JWT 立即失效。
3. **LLM 多轮对话 / Function Calling**：详见 §15.15。
4. **LLM Token 成本追踪**：累计 usage 到租户维度，支持成本统计和配额管控。
5. **通知渠道扩展**：邮件 (SMTP)、IM 直连（企业微信/钉钉/Slack）、WebSocket/SSE 实时推送。详见 §16.11。