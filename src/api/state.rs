use std::sync::Arc;

use futures::future::BoxFuture;

use crate::{AppResult, core::types::JobId};

use super::types::{
    CreateSourceRequest, ItemDto, JobDto, JobRunResponse, RunDto, SettingsDto, SourceDto,
    SyncErrorDto,
};

pub type ApiFuture<'a, T> = BoxFuture<'a, AppResult<T>>;

pub trait ApiRepository: Send + Sync {
    fn list_sources(&self) -> ApiFuture<'_, Vec<SourceDto>>;

    fn create_source(&self, request: CreateSourceRequest) -> ApiFuture<'_, SourceDto>;

    fn list_jobs(&self) -> ApiFuture<'_, Vec<JobDto>>;

    fn list_runs(&self) -> ApiFuture<'_, Vec<RunDto>>;

    fn list_items(&self) -> ApiFuture<'_, Vec<ItemDto>>;

    fn list_errors(&self) -> ApiFuture<'_, Vec<SyncErrorDto>>;

    fn settings(&self) -> ApiFuture<'_, SettingsDto>;
}

pub trait SyncService: Send + Sync {
    fn run_job(&self, job_id: JobId) -> ApiFuture<'_, JobRunResponse>;
}

#[derive(Clone)]
pub struct ApiState {
    repository: Arc<dyn ApiRepository>,
    sync_service: Arc<dyn SyncService>,
}

impl ApiState {
    pub fn new(repository: Arc<dyn ApiRepository>, sync_service: Arc<dyn SyncService>) -> Self {
        Self {
            repository,
            sync_service,
        }
    }

    pub fn repository(&self) -> &dyn ApiRepository {
        self.repository.as_ref()
    }

    pub fn sync_service(&self) -> &dyn SyncService {
        self.sync_service.as_ref()
    }
}
