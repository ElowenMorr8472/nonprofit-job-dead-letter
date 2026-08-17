use nonprofit_job_dead_letter::{
    infrai_queue::{InfraiError, InfraiQueue},
    nonprofit_job::{decide_failure, DeadLetter, FailedJob, FailureDecision},
};
use thiserror::Error;

const MAX_JOB_ATTEMPTS: u8 = 3;
const SOURCE_QUEUE: &str = "nonprofit-jobs";
const DEAD_LETTER_QUEUE: &str = "nonprofit-jobs-dead-letter";

#[derive(Debug, Error)]
enum WorkerError {
    #[error(transparent)]
    Queue(#[from] InfraiError),
}

#[tokio::main]
async fn main() -> Result<(), WorkerError> {
    let queue = InfraiQueue::from_env()?;
    let messages = queue.consume::<FailedJob>(SOURCE_QUEUE, 10, 60).await?;

    for message in messages {
        match decide_failure(message.payload.attempts, MAX_JOB_ATTEMPTS) {
            FailureDecision::Retry => {
                println!(
                    "retry {} after attempt {}",
                    message.payload.job.job_key(),
                    message.payload.attempts
                );
            }
            FailureDecision::DeadLetter => {
                let key = format!("dead-letter-{}", message.message_id);
                let letter = DeadLetter {
                    source_message_id: message.message_id.clone(),
                    failed: message.payload,
                };
                queue.publish(DEAD_LETTER_QUEUE, &letter, &key).await?;
                queue.ack(SOURCE_QUEUE, &message.message_id).await?;
                println!("dead-lettered {}", letter.failed.job.job_key());
            }
        }
    }
    Ok(())
}
