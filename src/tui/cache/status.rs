#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStatus {
    Idle,
    Loading,
    Revalidating,
    Ready,
    Failed(String),
}

impl CacheStatus {
    pub fn in_flight(&self) -> bool {
        match self {
            CacheStatus::Loading | CacheStatus::Revalidating => true,
            CacheStatus::Idle | CacheStatus::Ready | CacheStatus::Failed(_) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Skip,
    Load,
    Revalidate,
    Bust,
}
