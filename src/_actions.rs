use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::manager::{ContextTrait, EscalonJobsManager};
use crate::{EscalonJob, EscalonJobStatus, EscalonJobTrait, NewEscalonJob};

impl<T: ContextTrait<T> + Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    pub async fn create_job(
        &self,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> Uuid {
        let new_job = new_cron_job.clone().into();
        let manager = self.clone();

        let job = Job::new_async(new_job.schedule.as_str(), move |uuid, lock| {
            let new_job = new_cron_job.clone();
            let manager = manager.clone();

            Box::pin(async move {
                manager.status_handler(uuid, lock, new_job).await;
            })
        })
        .unwrap();

        let scheduler = self.scheduler.lock().unwrap().clone();
        scheduler.add(job).await.unwrap()
    }

    async fn spawn_run_job(
        &self,
        job_id: Uuid,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) {
        let job = self.get_job(job_id).await;

        if let Some(job) = job {
            let es_job = new_cron_job.run_job(&self.context.clone(), job.clone()).await;

            if es_job != job {
                self.update_status(job_id, es_job.status.to_owned()).await;
                // self.context.update_job(&self.context, es_job).await;
            }
        };
    }

    async fn status_handler(
        &self,
        uuid: Uuid,
        mut lock: JobScheduler,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) {
        let new_job = new_cron_job.clone().into();
        let next_tick = lock.next_tick_for_job(uuid).await.unwrap().unwrap().naive_utc();

        let status =
            self.jobs.lock().unwrap().iter().find(|j| j.job_id == uuid).unwrap().status.clone();

        match status {
            EscalonJobStatus::Scheduled => {
                if let Some(since) = new_job.since {
                    if since > next_tick {
                        return;
                    }
                }

                // if let Some(job) = self.get_job(uuid).await {
                if (self.get_job(uuid).await).is_some() {
                    self.update_status(uuid, EscalonJobStatus::Running).await;
                    // self.context.update_job(&self.context, job).await;
                }

                let manager = self.clone();
                escalon::tokio::task::spawn(async move {
                    manager.spawn_run_job(uuid, new_cron_job).await;
                });
                // self.spawn_run_job(uuid, new_cron_job).await;
            }
            EscalonJobStatus::Running => {
                let manager = self.clone();
                escalon::tokio::task::spawn(async move {
                    manager.spawn_run_job(uuid, new_cron_job).await;
                });
                // self.spawn_run_job(uuid, new_cron_job).await;
            }
            EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                // This way ensures that the job is in the scheduler and then removes it
                if let Some(job) = self.get_job(uuid).await {
                    self.remove_job(job.job_id).await;
                }
            }
        }

        if let Some(until) = new_job.until {
            if until < next_tick {
                // if let Some(job) = self.get_job(uuid).await {
                if (self.get_job(uuid).await).is_some() {
                    self.update_status(uuid, EscalonJobStatus::Done).await;
                    // self.context.update_job(&self.context, job).await;
                }
            }
        }
    }

    async fn update_status(&self, id: Uuid, status: EscalonJobStatus) {
        let out: EscalonJob;
        {
            let mut jobs = self.jobs.lock().unwrap();

            let job = jobs.iter_mut().find(|j| j.job_id == id).unwrap();
            job.status = status;

            out = job.clone();
        }

        self.context.update_job(&self.context, out).await;
    }
}
