use icu::segmenter::GraphemeClusterSegmenter;
use secrecy::{ExposeSecret, SecretString};

const MIN_PASSWORD_LENGTH: usize = 12;
const MAX_PASSWORD_LENGTH: usize = 256;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum InvalidPasswordError {
    #[error("Password is empty")]
    Empty,
    #[error("Password is too short")]
    TooShort,
    #[error("Password is too long")]
    TooLong,
}

#[derive(Debug)]
pub struct Password(SecretString);

impl Password {
    pub fn parse(s: String) -> Result<Self, InvalidPasswordError> {
        if s.is_empty() {
            return Err(InvalidPasswordError::Empty);
        }

        // segment_str returns the breakpoints, subtract 1 to grapheme cluster count
        let len = GraphemeClusterSegmenter::new().segment_str(&s).count() - 1;
        if len < MIN_PASSWORD_LENGTH {
            return Err(InvalidPasswordError::TooShort);
        }
        if len > MAX_PASSWORD_LENGTH {
            return Err(InvalidPasswordError::TooLong);
        }

        Ok(Self(SecretString::from(s)))
    }
}

impl ExposeSecret<str> for Password {
    fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::password::{InvalidPasswordError, Password};
    use claims::{assert_err_eq, assert_ok};

    #[test]
    fn empty_password_is_invalid() {
        let passwd = "".to_string();
        assert_err_eq!(Password::parse(passwd), InvalidPasswordError::Empty);
    }

    #[test]
    fn a_11_grapheme_long_password_is_invalid() {
        let passwd = "ё".repeat(11);
        assert_err_eq!(Password::parse(passwd), InvalidPasswordError::TooShort);
    }

    #[test]
    fn a_12_grapheme_long_password_is_valid() {
        let passwd = "ё".repeat(12);
        assert_ok!(Password::parse(passwd));
    }

    #[test]
    fn a_257_grapheme_long_password_is_invalid() {
        let passwd = "ё".repeat(257);
        assert_err_eq!(Password::parse(passwd), InvalidPasswordError::TooLong);
    }

    #[test]
    fn a_256_grapheme_long_password_is_valid() {
        let passwd = "ё".repeat(256);
        assert_ok!(Password::parse(passwd));
    }
}
