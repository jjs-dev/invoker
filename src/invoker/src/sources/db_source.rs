use crate::controller::{InvocationFinishReason, TaskSource};
use anyhow::Context;
use std::path::PathBuf;
use uuid::Uuid;

pub struct DbSource {
    db: db::DbConn,
    runs_dir: PathBuf,
    run_mapping: tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, db::schema::RunId>>,
}

impl DbSource {
    pub fn new(db: db::DbConn, cfg_data: &util::cfg::CfgData) -> DbSource {
        DbSource {
            db,
            runs_dir: cfg_data.data_dir.join("var/runs"),
            run_mapping: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl TaskSource for DbSource {
    async fn load_tasks(&self, mut cnt: usize) -> anyhow::Result<Vec<invoker_api::InvokeTask>> {
        let mut new_tasks = Vec::new();
        const WINDOW_SIZE: u32 = 10;
        const WINDOW_STEP: u32 = 9;
        {
            #[allow(dead_code)]
            const ASSERT_SIZE_IS_GREATER_THAN_STEP: usize =
                (WINDOW_SIZE - WINDOW_STEP - 1) as usize;
        }
        let mut offset = 0;
        while cnt > 0 {
            let mut discovered_new_tasks = false;
            let chunk: Vec<db::schema::Invocation> = self
                .db
                .inv_find_waiting(offset, WINDOW_SIZE, &mut |_invocation| {
                    if cnt > 0 {
                        discovered_new_tasks = true;
                        cnt -= 1;
                        return Ok(true);
                    }
                    Ok(false)
                })
                .await?;
            let mut mapping = self.run_mapping.lock().await;
            for invocation in chunk {
                let db_invoke_task = invocation.invoke_task()?;
                let db_run = self.db.run_load(db_invoke_task.run_id as i32).await?;
                let invocation_id = Uuid::from_fields(invocation.id as u32, 0, 0, &[0; 8])
                    .expect("this call is always correct");
                let run_dir = self
                    .runs_dir
                    .join(&format!("run.{}", db_invoke_task.run_id));
                let invocation_dir = run_dir.join(&format!("inv.{}", db_invoke_task.revision));
                let invoke_task = invoker_api::InvokeTask {
                    revision: db_invoke_task.revision,
                    toolchain_id: db_run.toolchain_id,
                    problem_id: db_run.problem_id,
                    invocation_id,
                    run_dir,
                    invocation_dir,
                };
                mapping.insert(invocation_id, db_invoke_task.run_id as i32);
                new_tasks.push(invoke_task);
            }
            if !discovered_new_tasks {
                break;
            }
            offset += WINDOW_STEP;
        }
        Ok(new_tasks)
    }

    async fn set_finished(
        &self,
        invocation_id: uuid::Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        self.run_mapping.lock().await.remove(&invocation_id);
        let mut patch = db::schema::InvocationPatch::default();
        let state = match reason {
            InvocationFinishReason::CompileError => db::schema::InvocationState::CompileError,
            InvocationFinishReason::Fault => db::schema::InvocationState::InvokeFailed,
            InvocationFinishReason::JudgeDone => db::schema::InvocationState::JudgeDone,
        };
        patch.state(state);
        self.db
            .inv_update(invocation_id.as_fields().0 as i32, patch)
            .await
            .context("failed to store outcome")?;
        Ok(())
    }

    async fn add_outcome_header(
        &self,
        invocation_id: uuid::Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        self.db
            .inv_add_outcome_header(invocation_id.as_fields().0 as i32, header)
            .await
    }

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        lsu: invoker_api::LiveStatusUpdate,
    ) -> anyhow::Result<()> {
        let run_id = match self.run_mapping.lock().await.get(&invocation_id) {
            Some(id) => *id,
            None => {
                anyhow::bail!("warning: invocation_id {} not found", invocation_id);
            }
        };
        let key = format!("lsu-{}", run_id);
        self.db.kv_put(&key, lsu).await
    }
}