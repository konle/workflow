use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::task::entity::{HttpMethod, TaskHttpTemplate, TaskTemplate};
use crate::workflow::entity::{
    NodeExecutionStatus, NodeOutput, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct HttpPlugin {
    client: Client,
}

impl HttpPlugin {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn do_request(&self, config: &TaskHttpTemplate) -> anyhow::Result<(NodeExecutionStatus, Option<NodeOutput>, Option<String>)> {
        let mut last_error: Option<String> = None;
        let attempts = config.retry_count + 1;

        for attempt in 0..attempts {
            let mut request = match config.method {
                HttpMethod::Get => self.client.get(&config.url),
                HttpMethod::Post => self.client.post(&config.url),
                HttpMethod::Put => self.client.put(&config.url),
                HttpMethod::Delete => self.client.delete(&config.url),
                HttpMethod::Head => self.client.head(&config.url),
            };

            for (k, v) in &config.headers {
                request = request.header(k.as_str(), v.as_str());
            }

            if let Some(ref form) = config.body {
                request = request.json(form);
            }

            if config.timeout > 0 {
                request = request.timeout(std::time::Duration::from_secs(config.timeout as u64));
            }

            let start = Utc::now();
            match request.send().await {
                Ok(resp) => {
                    let status_code = resp.status().as_u16();
                    let resp_body = resp.text().await.unwrap_or_default();
                    let duration = (Utc::now() - start).num_milliseconds().max(0) as u64;

                    let output_data = json!({
                        "status_code": status_code,
                        "body": resp_body,
                        "duration_ms": duration,
                        "attempt": attempt + 1,
                    });

                    let output = Some(NodeOutput { data: output_data });

                    if (200..300).contains(&status_code) {
                        return Ok((NodeExecutionStatus::Success, output, None));
                    } else {
                        last_error = Some(format!("HTTP {}: {}", status_code, resp_body));
                    }
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }

            if attempt < config.retry_count {
                let delay = config.retry_delay as u64;
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }

        let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
        Ok((NodeExecutionStatus::Failed, None, Some(error_msg)))
    }
}

#[async_trait]
impl PluginInterface for HttpPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        // 构造异步任务
        let job = ExecuteTaskJob {
            task_instance_id: format!("{}-{}", workflow_instance.workflow_instance_id, node_instance.node_id),
            tenant_id: "default".to_string(), // TODO: 从上下文中获取
            caller_context: Some(WorkflowCallerContext {
                workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                node_id: node_instance.node_id.clone(),
            }),
        };

        // 返回 AsyncDispatch，让 Manager 挂起工作流并投递任务
        Ok(ExecutionResult::async_dispatch(job))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Http
    }
}
