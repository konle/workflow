use crate::shared::form::Form;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt::{self, Display};
use crate::shared::workflow::{TaskStatus, TaskInstanceStatus, TaskType};






// 任务模板枚举 用于表示任务的模板 如http、grpc、审批等
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskTemplate {
    Http(TaskHttpTemplate),
    Grpc,
    Approval,
    IfCondition,

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
    pub timeout: u32, // 超时时间 单位：秒
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head
}

impl Display for TaskTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEntity {
    pub id: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInstanceEntity {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub task_type: TaskType,
    pub task_template: TaskTemplate,
    pub task_status: TaskInstanceStatus,
    pub task_instance_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub output: Option<JsonValue>,
    pub error_message: Option<String>,
    pub execution_duration: Option<u64>, // 执行时间 单位：毫秒
}