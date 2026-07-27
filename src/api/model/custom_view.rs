use serde::{Deserialize, Serialize};

use super::id::ViewId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedView {
    pub id: ViewId,
    pub name: String,
}
