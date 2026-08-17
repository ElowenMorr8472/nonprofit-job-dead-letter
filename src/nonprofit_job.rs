use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NonprofitJob {
    DonorReceipt { donation_id: String, donor_id: String },
    VolunteerReminder { shift_id: String, volunteer_id: String },
    CampaignReport { campaign_id: String, period: String },
}

impl NonprofitJob {
    pub fn job_key(&self) -> &str {
        match self {
            Self::DonorReceipt { donation_id, .. } => donation_id,
            Self::VolunteerReminder { shift_id, .. } => shift_id,
            Self::CampaignReport { campaign_id, .. } => campaign_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedJob {
    pub job: NonprofitJob,
    pub attempts: u8,
    pub last_error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeadLetter {
    pub source_message_id: String,
    pub failed: FailedJob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDecision {
    Retry,
    DeadLetter,
}

pub fn decide_failure(attempts: u8, max_attempts: u8) -> FailureDecision {
    if attempts >= max_attempts {
        FailureDecision::DeadLetter
    } else {
        FailureDecision::Retry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_failure_dead_letters_each_nonprofit_job_kind() {
        let jobs = [
            NonprofitJob::DonorReceipt {
                donation_id: "don-1042".into(),
                donor_id: "donor-8".into(),
            },
            NonprofitJob::VolunteerReminder {
                shift_id: "shift-21".into(),
                volunteer_id: "vol-5".into(),
            },
            NonprofitJob::CampaignReport {
                campaign_id: "campaign-spring".into(),
                period: "2026-Q2".into(),
            },
        ];

        for job in jobs {
            assert_eq!(decide_failure(2, 3), FailureDecision::Retry);
            assert_eq!(decide_failure(3, 3), FailureDecision::DeadLetter);
            let letter = DeadLetter {
                source_message_id: "msg-77".into(),
                failed: FailedJob {
                    job: job.clone(),
                    attempts: 3,
                    last_error: "downstream rejected the job".into(),
                },
            };
            assert_eq!(letter.failed.job.job_key(), job.job_key());
        }
    }
}
