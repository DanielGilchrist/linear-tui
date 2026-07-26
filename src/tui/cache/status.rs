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

    pub fn empty_placeholder<'a>(&self, empty: &'a str) -> &'a str {
        match self {
            CacheStatus::Loading => "Loading…",
            CacheStatus::Failed(_) => "Failed to load  ·  r to retry",
            CacheStatus::Idle | CacheStatus::Revalidating | CacheStatus::Ready => empty,
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
