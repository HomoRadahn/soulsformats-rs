use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The ID of this `Entry`
    pub id: i32,
    /// The text data of this `Entry`
    pub text: Option<String>,
}

impl Entry {
    /// Creates `Entry` with specified parameters
    pub fn new(id: i32, text: Option<impl Into<String>>) -> Self {
        Self {
            id,
            text: text.map(Into::into),
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.text {
            Some(text) => write!(f, "{}: {}", self.id, text),
            None => write!(f, "{}: <null>", self.id),
        }
    }
}
