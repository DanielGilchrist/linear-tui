#[derive(Debug, Clone, Copy)]
pub struct RefreshPolicy {
    pub fresh_for: i64,
    pub cold_after: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    Fresh,
    Stale,
    Cold,
}

impl RefreshPolicy {
    pub const fn new(fresh_for: i64, cold_after: i64) -> Self {
        Self {
            fresh_for,
            cold_after,
        }
    }

    pub fn classify(&self, age: i64) -> Age {
        if age < self.fresh_for {
            Age::Fresh
        } else if age < self.cold_after {
            Age::Stale
        } else {
            Age::Cold
        }
    }
}
