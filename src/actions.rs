use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::{EscalonJob, EscalonJobStatus, EscalonJobTrait, NewEscalonJob};
use crate::manager::EscalonJobsManager;

impl<T: Clone + Send + Sync + 'static> EscalonJobsManager<T> {
    pub async fn create_job(
        &self,
        new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static,
    ) -> EscalonJob {
        let cloned = new_cron_job.clone().into();
        let new_job = new_cron_job.clone();
        let manager = self.clone();

        let job =
            Job::new_async(new_job.into().schedule.clone().as_str(), move |uuid, lock| {
                let new_cron_job = new_cron_job.clone();
                let manager = manager.clone();

                Box::pin(async move {
                    manager.status_handler(uuid, &lock, new_cron_job).await;
                })
            })
            .unwrap();

        let job_id;
        {
            let scheduler = self.scheduler.lock().unwrap().clone();
            job_id = scheduler.add(job).await.unwrap();
        }

        let cron_job = EscalonJob {
            job_id,
            status: EscalonJobStatus::Scheduled,
            schedule: cloned.schedule,
            since: cloned.since,
            until: cloned.until,
        };

        self.jobs.lock().unwrap().push(cron_job.clone());

        cron_job
    }

    async fn status_handler(&self, uuid: Uuid, lock: &JobScheduler, new_cron_job: impl EscalonJobTrait<T> + Into<NewEscalonJob> + Clone + Send + Sync + 'static) {
        let status = self.jobs
            .lock()
            .unwrap()
            .iter()
            .find(|j| j.job_id == uuid)
            .unwrap()
            .status
            .clone();

        match status {
            EscalonJobStatus::Scheduled => {
                // check things like since and until
                // to change state to active

                let job = self.jobs
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|j| j.job_id == uuid)
                    .unwrap()
                    .clone();

                self.update_state(uuid, EscalonJobStatus::Running);
                new_cron_job.update_db(&job).await;
            }
            EscalonJobStatus::Running => {
                let job = self.jobs
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|j| j.job_id == uuid)
                    .unwrap()
                    .clone();

                let returned = new_cron_job.run(job.clone(), self.context.clone()).await;

                if returned != job {
                    self.update_state(uuid, returned.status.to_owned());
                }
            }
            EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                let job = self.jobs
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|j| j.job_id == uuid)
                    .unwrap()
                    .clone();

                lock.remove(&uuid).await.unwrap();
                new_cron_job.update_db(&job).await;
            }
        }
    }

    fn update_state(&self, uuid: Uuid, status: EscalonJobStatus) {
        self.jobs
            .lock()
            .unwrap()
            .iter_mut()
            .find(|j| j.job_id == uuid)
            .unwrap()
            .status = status.to_owned();
    }

}
