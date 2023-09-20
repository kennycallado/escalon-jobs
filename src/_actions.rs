use escalon::tokio as tokio;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::manager::{EscalonJobsManager, ContextTrait};
use crate::{EscalonJobStatus, EscalonJobTrait, NewEscalonJob};

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
                self.context.0.update_job(&self.get_job(uuid).await, &self.context.0);
            }
            EscalonJobStatus::Running => {
                // let job = self.get_job(uuid).await;
                // let es_job = new_cron_job.run_job(job.clone(), self.context.0.clone()).await;

                // if es_job != job {
                //     self.update_status(uuid, es_job.status.to_owned());
                //     self.context.0.update_job(&self.get_job(uuid).await, &self.context.0);
                // }

                let manager = self.clone();
                tokio::task::spawn(async move {
                    let job = manager.get_job(uuid).await;
                    let es_job = new_cron_job.run_job(job.clone(), manager.context.0.clone()).await;

                    if es_job != job {
                        manager.update_status(uuid, es_job.status.to_owned());
                        manager.context.0.update_job(&manager.get_job(uuid).await, &manager.context.0);
                    }
                });
            }
            EscalonJobStatus::Done | EscalonJobStatus::Failed => {
                lock.remove(&uuid).await.unwrap();
                self.context.0.update_job(&self.get_job(uuid).await, &self.context.0);
            }
        }

        if let Some(until) = new_job.until {
            if until < next_tick {
                self.update_status(uuid, EscalonJobStatus::Done);
                self.context.0.update_job(&self.get_job(uuid).await, &self.context.0);
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
