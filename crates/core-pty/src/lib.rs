pub const CRATE_NAME: &str = "core-pty";

#[must_use]
pub fn crate_name() -> &'static str {
    CRATE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_expected_name() {
        assert_eq!(crate_name(), CRATE_NAME);
    }
}
