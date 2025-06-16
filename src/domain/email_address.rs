use validator::ValidateEmail;

#[derive(thiserror::Error, Debug)]
#[error("Invalid email")]
pub struct InvalidEmailError;

#[derive(Debug)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn parse(s: &str) -> Result<EmailAddress, InvalidEmailError> {
        let s = s.to_lowercase();
        if (s).validate_email() {
            Ok(Self(s))
        } else {
            Err(InvalidEmailError)
        }
    }
}

impl std::fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
