use serde::{Deserialize, Serialize};

macro_rules! id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Fixtures and tests only. Real ids arrive as `cynic::Id` from the
            /// API, or through `Deserialize` when a cache is read back.
            pub fn from_raw(raw: impl Into<String>) -> Self {
                Self(raw.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<cynic::Id> for $name {
            fn from(id: cynic::Id) -> Self {
                Self(id.into_inner())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id!(IssueId);
id!(CommentId);
id!(TeamId);
id!(UserId);
id!(ViewId);
id!(StateId);
id!(ReactionId);
id!(LabelId);
