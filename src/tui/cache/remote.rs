use super::policy::{Age, RefreshPolicy};
use super::status::{Access, CacheStatus};
use crate::api::Timestamp;

pub trait Stale {
    fn mark_stale(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Remote<T> {
    value: Option<T>,
    status: CacheStatus,
    fetched_at: Timestamp,
}

impl<T> Default for Remote<T> {
    fn default() -> Self {
        Self {
            value: None,
            status: CacheStatus::Idle,
            fetched_at: Timestamp::default(),
        }
    }
}

impl<T> Remote<T> {
    pub fn ready(value: T, fetched_at: Timestamp) -> Self {
        Self {
            value: Some(value),
            status: CacheStatus::Ready,
            fetched_at,
        }
    }

    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }

    pub fn status(&self) -> &CacheStatus {
        &self.status
    }

    pub fn fetched_at(&self) -> Timestamp {
        self.fetched_at
    }

    pub fn in_flight(&self) -> bool {
        self.status.in_flight()
    }

    pub fn phase(&self) -> Phase {
        match (&self.value, &self.status) {
            (Some(_), _) => Phase::Ready,
            (None, CacheStatus::Loading | CacheStatus::Revalidating) => Phase::Loading,
            (None, CacheStatus::Failed(_)) => Phase::Failed,
            (None, CacheStatus::Idle | CacheStatus::Ready) => Phase::Missing,
        }
    }

    pub fn access(&self, now: Timestamp, policy: &RefreshPolicy) -> Access {
        if self.in_flight() {
            return Access::Skip;
        }
        match (
            &self.value,
            policy.classify(now.seconds_since(self.fetched_at)),
        ) {
            (None, _) => Access::Load,
            (Some(_), Age::Fresh) => Access::Skip,
            (Some(_), Age::Stale) => Access::Revalidate,
            (Some(_), Age::Cold) => Access::Bust,
        }
    }

    pub fn begin(&mut self) {
        self.status = match self.value {
            Some(_) => CacheStatus::Revalidating,
            None => CacheStatus::Loading,
        };
    }

    pub fn begin_access(&mut self, now: Timestamp, policy: &RefreshPolicy) -> bool {
        match self.access(now, policy) {
            Access::Skip => false,
            Access::Bust => {
                self.bust();
                self.begin();
                true
            }
            Access::Load | Access::Revalidate => {
                self.begin();
                true
            }
        }
    }

    pub fn set(&mut self, value: T, now: Timestamp) {
        self.value = Some(value);
        self.status = CacheStatus::Ready;
        self.fetched_at = now;
    }

    pub fn fail(&mut self, error: String) {
        self.status = CacheStatus::Failed(error);
    }

    pub fn bust(&mut self) {
        self.value = None;
        self.status = CacheStatus::Idle;
    }
}

impl<T> Stale for Remote<T> {
    fn mark_stale(&mut self) {
        self.fetched_at = Timestamp::default();
    }
}
