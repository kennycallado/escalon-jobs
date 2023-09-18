use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::manager::EscalonJobsManager;
use crate::{EscalonJobStatus, EscalonJobTrait, NewEscalonJob};

impl<T: Clone + Send + Sync + 'static> EscalonJobsManager<T> {
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

        let scheduler;
        {
            scheduler = self.scheduler.lock().unwrap().clone();
        }
        scheduler.add(job).await.unwrap()
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

                self.update_status(uuid, EscalonJobStatus::Running);
                new_cron_job.update_job(&self.get_job(uuid).await).await;
            }
            EscalonJobStatus::Running => {
                let job = self
                    .jobs
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .find(|j| j.job_id == uuid)
                    .unwrap()
                    .clone();

                let es_job = new_cron_job.run_job(job.clone(), self.context.clone()).await;

                if es_job != job {
                    self.update_status(uuid, es_job.status.to_owned());
                    new_cron_job.update_job(&es_job).await;
                }
            }
            EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                lock.remove(&uuid).await.unwrap();
                new_cron_job.update_job(&self.get_job(uuid).await).await;
            }
        }

        if let Some(until) = new_job.until {
            if until < next_tick {
                self.update_status(uuid, EscalonJobStatus::Done);
                new_cron_job.update_job(&self.get_job(uuid).await).await;
            }
        }
    }

    fn update_status(&self, id: Uuid, status: EscalonJobStatus) {
        let jobs = self.jobs.clone();

        {
            let mut jobs = jobs.lock().unwrap();
            let job = jobs.iter_mut().find(|j| j.job_id == id).unwrap();
            job.status = status;
        }
    }
}
