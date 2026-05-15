use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{task::JoinHandle, time};

use crate::{
    AppConfig, AppResult,
    api::types::{JobDto, JobScheduleDto},
    app::job_service,
    core::types::JobStatus,
    db::repository::{RuntimeSettingsRepository, SeaOrmRepository},
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
/// Returns an error when settings or jobs cannot be listed.
pub async fn run_due_jobs_once(
    repository: Arc<SeaOrmRepository>,
    config: &AppConfig,
) -> AppResult<usize> {
    let settings = repository.load_runtime_settings(config).await?;
    let job_concurrency = settings.job_concurrency;

    if job_concurrency == 0 {
        return Ok(0);
    }

    let jobs = job_service::list_jobs(repository.as_ref()).await?;
    let due_jobs = jobs
        .into_iter()
        .filter(is_due_interval_job)
        .take(job_concurrency);
    let mut started = 0;

    for job in due_jobs {
        match job_service::run_job(
            Arc::clone(&repository),
            config.vault_path.clone(),
            job.id,
            settings.file_concurrency,
        )
        .await
        {
            Ok(_) => {
                started += 1;
            }
            Err(error) => {
                tracing::warn!(
                    job_id = %job.id,
                    error = %error,
                    "scheduled job failed"
                );
            }
        }
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
