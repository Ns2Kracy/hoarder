use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{task::JoinHandle, time};

use crate::{
    AppConfig, AppResult,
    api::types::{JobDto, JobScheduleDto},
    app::job_service,
    core::types::JobStatus,
    db::repository::SeaOrmRepository,
};

/// Starts the fixed-interval scheduler loop.
#[must_use]
pub fn spawn_interval_scheduler(
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = time::interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if let Err(error) = run_due_jobs_once(Arc::clone(&repository), &config).await {
                tracing::warn!(error = %error, "scheduler tick failed");
            }
        }
    })
}

/// Runs one scheduler tick.
///
/// # Errors
///
/// Returns an error when listing jobs or running a due job fails.
pub async fn run_due_jobs_once(
    repository: Arc<SeaOrmRepository>,
    config: &AppConfig,
) -> AppResult<usize> {
    if config.job_concurrency == 0 {
        return Ok(0);
    }

    let jobs = job_service::list_jobs(repository.as_ref()).await?;
    let due_jobs = jobs
        .into_iter()
        .filter(is_due_interval_job)
        .take(config.job_concurrency);
    let mut started = 0;

    for job in due_jobs {
        job_service::run_job(Arc::clone(&repository), config.vault_path.clone(), job.id).await?;
        started += 1;
    }

    Ok(started)
}

fn is_due_interval_job(job: &JobDto) -> bool {
    if !job.enabled || job.status == JobStatus::Running {
        return false;
    }

    let JobScheduleDto::Interval { .. } = job.schedule else {
        return false;
    };

    job.next_run_at
        .is_none_or(|next_run_at| next_run_at <= Utc::now())
}
