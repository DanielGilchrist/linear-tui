#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(pub String);

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<Cursor>,
}

impl<T> Page<T> {
    pub fn single(items: Vec<T>) -> Self {
        Self { items, next: None }
    }
}
