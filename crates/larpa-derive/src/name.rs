use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct ArgNames {
    short: Option<char>,
    long: Option<String>,
}

impl ArgNames {
    pub fn new(
        short: impl Into<Option<char>>,
        long: impl Into<Option<String>>,
    ) -> Result<Self, String> {
        let short = short.into();
        let long = long.into();
        if short.is_none() && long.is_none() {
            return Err("either a short or a long argument name (or both) must be provided".into());
        }

        Ok(Self { short, long })
    }

    pub fn short(&self) -> Option<char> {
        self.short
    }

    pub fn long(&self) -> Option<&str> {
        self.long.as_deref()
    }
}

pub enum ArgName {
    Short(char),
    Long(String),
}

impl FromStr for ArgName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(long) = s.strip_prefix("--") {
            if long.is_empty() {
                return Err("`--` is not allowed as a long argument".into());
            }
            if long.contains('=') {
                return Err("long argument names must not contain `=`".into());
            }
            Ok(Self::Long(long.into()))
        } else if let Some(short) = s.strip_prefix('-') {
            if short.chars().count() != 1 {
                return Err("short argument name must consist of a single character".into());
            }
            let ch = short.chars().next().unwrap();
            Ok(Self::Short(ch))
        } else {
            Err("invalid argument name (format: `-s` or `--long`)".into())
        }
    }
}
