/*

Argument value parsing. We try these in order, using autoref specialization:

ArgValue        (covers larpa's built-in types)
TryFrom<&OsStr> (covers lossless conversion to `PathBuf`/`OsString`)
TryFrom<&str>   (covers `Box<str>`, `(A)Rc<str>`, `Vec<u8>`(?))
FromStr

All user error types must implement `Into<Box<dyn Error>>`.

*/

use std::{ffi::OsStr, marker::PhantomData, str::FromStr};

use crate::private::Context;

// FIXME: should this be `Send + Sync` too?
pub type BoxedError = Box<dyn std::error::Error + 'static>;

pub trait ArgValue: Sized {
    const CHOICES: &[&str] = &[];
    type Error: Into<BoxedError>;
    fn parse(s: &OsStr, cx: &Context) -> Result<Self, Self::Error>;
}

pub struct Converter<'a, T> {
    cx: &'a Context,
    _p: PhantomData<T>,
}

impl<'a, T> Converter<'a, T> {
    #[inline]
    pub fn new(cx: &'a Context) -> Self {
        Self {
            cx,
            _p: PhantomData,
        }
    }
}

pub trait ViaArgValue<T> {
    fn convert(self, s: &OsStr) -> Result<T, BoxedError>;
}
pub trait ViaTryFromOsStr<T> {
    fn convert(self, s: &OsStr) -> Result<T, BoxedError>;
}
pub trait ViaTryFromStr<T> {
    fn convert(self, s: &OsStr) -> Result<T, BoxedError>;
}
pub trait ViaFromStr<T> {
    fn convert(self, s: &OsStr) -> Result<T, BoxedError>;
}

// ArgValue
impl<T> ViaArgValue<T> for &&&&Converter<'_, T>
where
    T: ArgValue<Error: Into<BoxedError>>,
{
    fn convert(self, s: &OsStr) -> Result<T, BoxedError> {
        hit!(via_arg_value);
        T::parse(s, self.cx).map_err(Into::into)
    }
}

// TryFrom<&OsStr>
impl<T> ViaTryFromOsStr<T> for &&&Converter<'_, T>
where
    T: for<'a> TryFrom<&'a OsStr, Error: Into<BoxedError>>,
{
    fn convert(self, s: &OsStr) -> Result<T, BoxedError> {
        hit!(via_try_from_osstr);
        T::try_from(s).map_err(Into::into)
    }
}

// TryFrom<&str>
impl<T> ViaTryFromStr<T> for &&Converter<'_, T>
where
    T: for<'a> TryFrom<&'a str, Error: Into<BoxedError>>,
{
    fn convert(self, s: &OsStr) -> Result<T, BoxedError> {
        hit!(via_try_from_str);
        let s = str::from_utf8(s.as_encoded_bytes())?;
        T::try_from(s).map_err(Into::into)
    }
}

// FromStr
impl<T> ViaFromStr<T> for &Converter<'_, T>
where
    T: FromStr<Err: Into<BoxedError>>,
{
    fn convert(self, s: &OsStr) -> Result<T, BoxedError> {
        hit!(via_from_str);
        let s = str::from_utf8(s.as_encoded_bytes())?;
        T::from_str(s).map_err(Into::into)
    }
}

impl<T> Converter<'_, T> {
    // This fallback method exists for diagnostic purposes only. It is called when none of the
    // specialization traits above apply, and uses `#[on_unimplemented]` to provide better diagnostics.
    pub fn convert(self, _: &OsStr) -> Result<T, BoxedError>
    where
        T: NoArgValue,
    {
        unimplemented!()
    }
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as an argument value",
    note = "argument value types must implement `TryFrom<&OsStr>`, `TryFrom<&str>`, or `FromStr` \
        (and the error type must be convertible to `Box<dyn Error>`)"
)]
pub trait NoArgValue {}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use cov_mark::check;

    use super::*;

    macro_rules! convert {
        ($s:expr => $ty:ty) => {
            (&&&&Converter::<$ty>::new(&Context::dummy())).convert(OsStr::new($s))
        };
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ArgVal;
    impl ArgValue for ArgVal {
        type Error = String;
        fn parse(_s: &OsStr, _cx: &Context) -> Result<Self, Self::Error> {
            Ok(ArgVal)
        }
    }

    #[test]
    fn smoke() {
        {
            check!(via_arg_value);
            assert_eq!(convert!("123" => ArgVal).unwrap(), ArgVal);
        }

        {
            check!(via_try_from_osstr);
            assert_eq!(convert!("123" => OsString).unwrap(), OsString::from("123"));
        }

        {
            check!(via_try_from_osstr);
            assert_eq!(convert!("123" => PathBuf).unwrap(), PathBuf::from("123"));
        }

        {
            check!(via_try_from_str);
            assert_eq!(convert!("123" => String).unwrap(), String::from("123"));
        }
        {
            // This one's a bit questionable, but `Vec<u8>` implements `From<&str>`.
            check!(via_try_from_str);
            assert_eq!(convert!("123" => Vec<u8>).unwrap(), b"123".to_vec());
        }

        {
            check!(via_from_str);
            assert_eq!(convert!("123" => u8).unwrap(), 123);
        }
    }
}
