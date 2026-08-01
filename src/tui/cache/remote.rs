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

#[derive(Debug, Clone, Default)]
pub enum Remote<T> {
    #[default]
    Missing,
    Loading,
    Ready {
        value: T,
        fetched_at: Timestamp,
    },
    Stale {
        value: T,
        fetched_at: Timestamp,
    },
    Revalidating {
        value: T,
        fetched_at: Timestamp,
    },
    Failed {
        error: String,
        last: Option<(T, Timestamp)>,
    },
}

impl<T> Remote<T> {
    pub fn ready(value: T, fetched_at: Timestamp) -> Self {
        Remote::Ready { value, fetched_at }
    }

    pub fn value(&self) -> Option<&T> {
        match self {
            Remote::Ready { value, .. }
            | Remote::Stale { value, .. }
            | Remote::Revalidating { value, .. }
            | Remote::Failed {
                last: Some((value, _)),
                ..
            } => Some(value),
            Remote::Missing | Remote::Loading | Remote::Failed { last: None, .. } => None,
        }
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        match self {
            Remote::Ready { value, .. }
            | Remote::Stale { value, .. }
            | Remote::Revalidating { value, .. }
            | Remote::Failed {
                last: Some((value, _)),
                ..
            } => Some(value),
            Remote::Missing | Remote::Loading | Remote::Failed { last: None, .. } => None,
        }
    }

    pub fn status(&self) -> CacheStatus {
        match self {
            Remote::Missing => CacheStatus::Idle,
            Remote::Loading => CacheStatus::Loading,
            Remote::Ready { .. } | Remote::Stale { .. } => CacheStatus::Ready,
            Remote::Revalidating { .. } => CacheStatus::Revalidating,
            Remote::Failed { error, .. } => CacheStatus::Failed(error.clone()),
        }
    }

    pub fn fetched_at(&self) -> Timestamp {
        match self {
            Remote::Ready { fetched_at, .. }
            | Remote::Stale { fetched_at, .. }
            | Remote::Revalidating { fetched_at, .. }
            | Remote::Failed {
                last: Some((_, fetched_at)),
                ..
            } => *fetched_at,
            Remote::Missing | Remote::Loading | Remote::Failed { last: None, .. } => {
                Timestamp::default()
            }
        }
    }

    pub fn in_flight(&self) -> bool {
        matches!(self, Remote::Loading | Remote::Revalidating { .. })
    }

    pub fn phase(&self) -> Phase {
        match self {
            Remote::Missing => Phase::Missing,
            Remote::Loading => Phase::Loading,
            Remote::Ready { .. }
            | Remote::Stale { .. }
            | Remote::Revalidating { .. }
            | Remote::Failed { last: Some(_), .. } => Phase::Ready,
            Remote::Failed { last: None, .. } => Phase::Failed,
        }
    }

    pub fn access(&self, now: Timestamp, policy: &RefreshPolicy) -> Access {
        if self.in_flight() {
            return Access::Skip;
        }

        if matches!(self, Remote::Stale { .. }) {
            return Access::Revalidate;
        }

        match self.value() {
            None => Access::Load,
            Some(_) => match policy.classify(now.seconds_since(self.fetched_at())) {
                Age::Fresh => Access::Skip,
                Age::Stale => Access::Revalidate,
                Age::Cold => Access::Bust,
            },
        }
    }

    pub fn begin(&mut self) {
        *self = match std::mem::take(self) {
            Remote::Missing | Remote::Loading => Remote::Loading,
            Remote::Ready { value, fetched_at }
            | Remote::Stale { value, fetched_at }
            | Remote::Revalidating { value, fetched_at }
            | Remote::Failed {
                last: Some((value, fetched_at)),
                ..
            } => Remote::Revalidating { value, fetched_at },
            Remote::Failed { last: None, .. } => Remote::Loading,
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
        *self = Remote::Ready {
            value,
            fetched_at: now,
        };
    }

    pub fn fail(&mut self, error: String) {
        *self = match std::mem::take(self) {
            Remote::Ready { value, fetched_at }
            | Remote::Stale { value, fetched_at }
            | Remote::Revalidating { value, fetched_at } => Remote::Failed {
                error,
                last: Some((value, fetched_at)),
            },
            Remote::Failed { last, .. } => Remote::Failed { error, last },
            Remote::Missing | Remote::Loading => Remote::Failed { error, last: None },
        };
    }

    pub fn bust(&mut self) {
        *self = Remote::Missing;
    }

    pub fn cancel(&mut self) {
        *self = match std::mem::take(self) {
            Remote::Loading => Remote::Missing,
            Remote::Revalidating { value, fetched_at } => Remote::Ready { value, fetched_at },
            Remote::Missing => Remote::Missing,
            Remote::Ready { value, fetched_at } => Remote::Ready { value, fetched_at },
            Remote::Stale { value, fetched_at } => Remote::Stale { value, fetched_at },
            Remote::Failed { error, last } => Remote::Failed { error, last },
        };
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Remote::Failed { .. })
    }
}

impl<T> Stale for Remote<T> {
    fn mark_stale(&mut self) {
        *self = match std::mem::take(self) {
            Remote::Ready { value, fetched_at } | Remote::Stale { value, fetched_at } => {
                Remote::Stale { value, fetched_at }
            }
            Remote::Revalidating { value, fetched_at } => {
                Remote::Revalidating { value, fetched_at }
            }
            Remote::Failed { error, last } => Remote::Failed { error, last },
            Remote::Missing => Remote::Missing,
            Remote::Loading => Remote::Loading,
        };
    }
}
