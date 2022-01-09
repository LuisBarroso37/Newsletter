#[derive(Debug)]
pub struct Password(String);

impl Password {
    pub fn parse(s: String) -> Result<Self, String> {
        let too_short = s.len() < 12;
        let too_long = s.len() >= 128;

        if too_short {
            Err("New password should be at least 12 characters long".to_string())
        } else if too_long {
            Err("New password should be less than 128 characters long".to_string())
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use crate::domain::Password;
    use claim::{assert_err, assert_ok};

    #[test]
    fn password_is_too_short() {
        let password = "a".repeat(11);
        assert_err!(Password::parse(password));
    }

    #[test]
    fn password_is_too_long() {
        let password = "a".repeat(128);
        assert_err!(Password::parse(password));
    }

    #[test]
    fn password_is_valid() {
        let password = "a".repeat(127);
        assert_ok!(Password::parse(password));
    }
}
