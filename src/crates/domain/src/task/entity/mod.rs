use crate::shared::form::Form;
use crate::shared::workflow::{TaskInstanceStatus, TaskStatus, TaskType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt::{self, Display};

// 任务模板枚举 用于表示任务的模板 如http、grpc、审批等
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskTemplate {
    Http(TaskHttpTemplate),
    Grpc,
    Approval(ApprovalTemplate),
    IfCondition(IfConditionTemplate),
    ContextRewrite(ContextRewriteTemplate),
    Parallel(ParallelTemplate),
    ForkJoin(ForkJoinTemplate),
    SubWorkflow(SubWorkflowTemplate),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelMode {
    /// 滚动执行（类似线程池）：只要有完成的，就立刻从队列里拿新的补上，保持并发度跑满
    Rolling,
    /// 批量执行（类似分批）：以 `concurrency` 为一批，这批全部完成后，再启动下一批
    Batch,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelTemplate {
    /// 迭代目标路径 (例如: "data.users")，引擎会用类似 JsonPath 提取出一个 JSON 数组
    pub items_path: String,

    /// 迭代变量名 (例如: "user")。在子任务的模板中，可以通过 {{ user.name }} 和 {{ user.age }} 引用
    pub item_alias: String,
    /// 真正要被重复执行的那个原子任务的模板配置
    pub task_template: Box<TaskTemplate>,
    /// 并发度，即同时最多有多少个子任务在执行，默认为 10
    pub concurrency: u32,

    /// 并发模式：滚动(Rolling) 或 批量(Batch)
    pub mode: ParallelMode,
    /// 最大容忍的失败数量。
    /// - 若设为 0，则任何一个子任务失败，整个并发容器立刻判定为失败，并尝试取消其他进行中的任务。
    /// - 若设为 N，则允许最多 N 个任务失败。超过 N 时整体失败。
    /// - 若为 None，则无论失败多少个，都坚持执行完所有任务。
    pub max_failures: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkJoinTemplate {
    pub tasks: Vec<ForkJoinTaskItem>,
    pub concurrency: u32,
    pub mode: ParallelMode,
    pub max_failures: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkJoinTaskItem {
    pub task_key: String,
    pub name: String,
    pub task_template: TaskTemplate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubWorkflowTemplate {
    pub workflow_meta_id: String,
    pub workflow_version: u32,
    pub input_mapping: Option<JsonValue>,
    pub output_path: Option<String>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskHttpTemplate {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Form>,
    pub form: Option<Form>,
    pub retry_count: u32, // 重试次数
    pub retry_delay: u32, // 重试延迟 单位：秒
    pub timeout: u32,     // 超时时间 单位：秒
    // 添加成功条件检测 如状态码、响应体、正则表达式等
    pub success_condition: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfConditionTemplate {
    pub condition: String,
    pub name: String,
    pub then_task: Option<String>,
    pub else_task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeMode {
    Merge,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextRewriteTemplate {
    pub name: String,
    pub script: String,
    #[serde(default = "default_merge_mode")]
    pub merge_mode: MergeMode,
}

fn default_merge_mode() -> MergeMode {
    MergeMode::Merge
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalTemplate {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub approvers: Vec<ApproverRule>,
    #[serde(default = "default_approval_mode")]
    pub approval_mode: ApprovalMode,
    pub timeout: Option<u64>,
}

fn default_approval_mode() -> ApprovalMode {
    ApprovalMode::Any
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApproverRule {
    User(String),
    Role(String),
    ContextVariable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalMode {
    Any,
    All,
    Majority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
}

impl Display for TaskTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEntity {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub task_type: TaskType,
    pub task_template: TaskTemplate,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl TaskEntity {
    pub fn new(
        id: String,
        tenant_id: String,
        name: String,
        task_type: TaskType,
        task_template: TaskTemplate,
        description: String,
        status: TaskStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        deleted_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name,
            task_type,
            task_template,
            description,
            status,
            created_at,
            updated_at,
            deleted_at,
        }
    }
}

use crate::shared::job::WorkflowCallerContext;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInstanceEntity {
    pub id: String,
    pub tenant_id: String,
    pub task_id: String,
    pub task_name: String,
    pub task_type: TaskType,
    pub task_template: TaskTemplate,
    pub task_status: TaskInstanceStatus,
    pub task_instance_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub input: Option<JsonValue>, // 保存请求的输入数据 比如HTTP渲染之后的http请求等
    pub output: Option<JsonValue>, // 保存请求响应的结果
    pub error_message: Option<String>,
    pub execution_duration: Option<u64>, // 执行时间 单位：毫秒
    pub caller_context: Option<WorkflowCallerContext>,
}
