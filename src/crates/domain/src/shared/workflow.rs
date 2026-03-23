use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Draft,
    Published,
    Deleted,
    Archived,
}
impl Display for WorkflowStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WorkflowStatus {
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            "draft" => Some(WorkflowStatus::Draft),
            "published" => Some(WorkflowStatus::Published),
            "deleted" => Some(WorkflowStatus::Deleted),
            _ => None,
        }
    }
}

// 工作流实例状态枚举 用于表示工作流实例的状态 如待执行、执行中、已完成、失败
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowInstanceStatus {
    Pending,
    Running,
    Await,
    Completed,
    Failed,
    Canceled,
    Suspended,
}
impl Display for WorkflowInstanceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl WorkflowInstanceStatus {
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            "pending" => Some(WorkflowInstanceStatus::Pending),
            "running" => Some(WorkflowInstanceStatus::Running),
            "completed" => Some(WorkflowInstanceStatus::Completed),
            "failed" => Some(WorkflowInstanceStatus::Failed),
            "canceled" => Some(WorkflowInstanceStatus::Canceled),
            "suspended" => Some(WorkflowInstanceStatus::Suspended),
            "await" => Some(WorkflowInstanceStatus::Await),
            _ => None,
        }
    }

    /// State machine transition rules:
    ///   Pending   -> Running
    ///   Running   -> Completed | Failed | Suspended | Await
    ///   Failed    -> Pending (retry) | Canceled
    ///   Suspended -> Running (resume) | Canceled
    ///   Completed -> (terminal)
    ///   Canceled  -> (terminal)
    ///   Await     -> Pending
    pub fn can_transition_to(&self, target: &WorkflowInstanceStatus) -> bool {
        matches!(
            (self, target),
            (WorkflowInstanceStatus::Pending, WorkflowInstanceStatus::Running)
                | (WorkflowInstanceStatus::Running, WorkflowInstanceStatus::Completed)
                | (WorkflowInstanceStatus::Running, WorkflowInstanceStatus::Failed)
                | (WorkflowInstanceStatus::Running, WorkflowInstanceStatus::Suspended)
                | (WorkflowInstanceStatus::Running, WorkflowInstanceStatus::Await)
                | (WorkflowInstanceStatus::Failed, WorkflowInstanceStatus::Pending)
                | (WorkflowInstanceStatus::Failed, WorkflowInstanceStatus::Canceled)
                | (WorkflowInstanceStatus::Suspended, WorkflowInstanceStatus::Running)
                | (WorkflowInstanceStatus::Suspended, WorkflowInstanceStatus::Canceled)
                | (WorkflowInstanceStatus::Await, WorkflowInstanceStatus::Pending)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowInstanceStatus::Completed | WorkflowInstanceStatus::Canceled)
    }
}

// 任务状态枚举 用于表示任务模板的状态 如草稿和已发布状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Draft,
    Published,
}
impl Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TaskStatus {
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            "draft" => Some(TaskStatus::Draft),
            "published" => Some(TaskStatus::Published),
            _ => None,
        }
    }
}

// 任务实例状态枚举 用于表示任务实例的状态 如待执行、执行中、已完成、失败
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskInstanceStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
}
impl Display for TaskInstanceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TaskInstanceStatus {
    pub fn from_str(status: &str) -> Option<Self> {
        match status {
            "pending" => Some(TaskInstanceStatus::Pending),
            "running" => Some(TaskInstanceStatus::Running),
            "completed" => Some(TaskInstanceStatus::Completed),
            "failed" => Some(TaskInstanceStatus::Failed),
            "canceled" => Some(TaskInstanceStatus::Canceled),
            _ => None,
        }
    }

    /// State machine transition rules:
    ///   Pending   -> Running | Canceled
    ///   Running   -> Completed | Failed
    ///   Failed    -> Pending (retry) | Canceled
    ///   Completed -> (terminal)
    ///   Canceled  -> (terminal)
    pub fn can_transition_to(&self, target: &TaskInstanceStatus) -> bool {
        matches!(
            (self, target),
            (TaskInstanceStatus::Pending, TaskInstanceStatus::Running)
                | (TaskInstanceStatus::Pending, TaskInstanceStatus::Canceled)
                | (TaskInstanceStatus::Running, TaskInstanceStatus::Completed)
                | (TaskInstanceStatus::Running, TaskInstanceStatus::Failed)
                | (TaskInstanceStatus::Failed, TaskInstanceStatus::Pending)
                | (TaskInstanceStatus::Failed, TaskInstanceStatus::Canceled)
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskInstanceStatus::Completed | TaskInstanceStatus::Canceled)
    }
}
// 任务类型枚举 用于表示任务的类型 如http、grpc、审批等
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    Http,
    IfCondition,
    Parallel,
    Grpc,
    Approval,
}