use icu::segmenter::GraphemeClusterSegmenter;

const MAX_USER_NAME_LENGTH: usize = 64;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum InvalidUsernameError {
    #[error("Empty username")]
    Empty,
    #[error("Username too long")]
    TooLong,
    #[error("Username contains forbidden character")]
    ContainsForbiddenCharacter,
}

#[derive(Debug, Clone)]
pub struct Username(String);

impl Username {
    pub fn parse(s: &str) -> Result<Username, InvalidUsernameError> {
        let username = s.trim().to_lowercase();

        if username.is_empty() {
            return Err(InvalidUsernameError::Empty);
        }

        // segment_str returns breakpoints, subtract 1 to get grapheme cluster count
        let len = GraphemeClusterSegmenter::new()
            .segment_str(&username)
            .count()
            - 1;
        if len > MAX_USER_NAME_LENGTH {
            return Err(InvalidUsernameError::TooLong);
        }

        if username
            .chars()
            .any(|g| !(g.is_ascii_alphanumeric() || g == '-' || g == '_' || g == '.'))
        {
            return Err(InvalidUsernameError::ContainsForbiddenCharacter);
        }

        Ok(Self(username))
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err_eq, assert_ok};

    use crate::domain::username::{InvalidUsernameError, Username};

    #[test]
    pub fn empty_username_is_invalid() {
        let username = "";
        assert_err_eq!(Username::parse(username), InvalidUsernameError::Empty);

        let username = " ";
        assert_err_eq!(Username::parse(username), InvalidUsernameError::Empty);
    }

    #[test]
    pub fn username_is_parsed_as_lowercase() {
        let username = "tEStUSer";
        assert_eq!(Username::parse(username).unwrap().as_ref(), "testuser");
    }

    #[test]
    pub fn a_65_grapheme_long_username_is_invalid() {
        let username = "a".repeat(65);
        assert_err_eq!(Username::parse(&username), InvalidUsernameError::TooLong);
    }

    #[test]
    pub fn a_64_grapheme_long_username_is_valid() {
        let username = "a".repeat(64);
        assert_ok!(Username::parse(&username));
    }

    #[test]
    pub fn username_containing_non_ascii_characters_is_invalid() {
        let username = "ё";
        assert_err_eq!(
            Username::parse(username),
            InvalidUsernameError::ContainsForbiddenCharacter
        );
    }

    #[test]
    pub fn username_containing_dash_underscore_dot_is_valid() {
        let username = ".";
        assert_ok!(Username::parse(username));

        let username = "_";
        assert_ok!(Username::parse(username));

        let username = "-";
        assert_ok!(Username::parse(username));
    }

    #[test]
    pub fn username_containing_forbidden_characters_is_invalid() {
        let username = "a a";
        assert_err_eq!(
            Username::parse(username),
            InvalidUsernameError::ContainsForbiddenCharacter
        );

        let username = "!".to_string();
        assert_err_eq!(
            Username::parse(&username),
            InvalidUsernameError::ContainsForbiddenCharacter
        );
    }
}
