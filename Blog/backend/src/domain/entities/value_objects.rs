use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn parse(value: &str) -> Result<Self, String> {
        let re = Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
        if re.is_match(value) {
            Ok(Self(value.to_lowercase()))
        } else {
            Err("Invalid email format".into())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedPassword(String);

impl HashedPassword {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
