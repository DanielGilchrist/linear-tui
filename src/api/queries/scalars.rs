use super::schema;

#[derive(cynic::Scalar, Debug, Clone)]
pub struct DateTime(pub String);
