use std::fmt::{Debug, Display};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct CustomError {
    message: String,
}

impl CustomError {
    pub fn new(message: String) -> Box<Self> {
        Box::new(CustomError { message })
    }
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl std::error::Error for CustomError {}

pub fn ensure_eq<T>(actual: T, expected: T, message: &str) -> Result<T>
where
    T: Eq + Display,
{
    if !(actual == expected) {
        Err(CustomError::new(format!(
            "{}: expected {}, found {}",
            message, expected, actual
        )))
    } else {
        Ok(actual)
    }
}

pub fn ensure_contains<T>(actual: T, expected: &[T], message: &str) -> Result<T>
where
    T: Eq + Display + Debug,
{
    if !expected.contains(&actual) {
        Err(CustomError::new(format!(
            "{}: expected {:?}, found {}",
            message, expected, actual
        )))
    } else {
        Ok(actual)
    }
}
