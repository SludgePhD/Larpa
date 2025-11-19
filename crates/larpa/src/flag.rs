use crate::private::Context;

pub trait Sealed {}
pub trait FlagInternal: Flag {
    fn set(&mut self, _cx: &Context);
}

#[diagnostic::on_unimplemented(message = "`{Self}` cannot be used as the target of `inverse_of`")]
pub trait InvertibleFlag: Flag {
    fn unset(&mut self, _cx: &Context);
}

impl Sealed for bool {}
impl FlagInternal for bool {
    #[inline]
    fn set(&mut self, _cx: &Context) {
        *self = true;
    }
}
impl InvertibleFlag for bool {
    #[inline]
    fn unset(&mut self, _cx: &Context) {
        *self = false;
    }
}

impl Sealed for Option<bool> {}
impl FlagInternal for Option<bool> {
    #[inline]
    fn set(&mut self, _cx: &Context) {
        *self = Some(true);
    }
}
impl InvertibleFlag for Option<bool> {
    #[inline]
    fn unset(&mut self, _cx: &Context) {
        *self = Some(false);
    }
}

/// Types that track the presence of a command-line flag without argument.
///
/// This trait is implemented for types that can be used with [`#[larpa(flag)]`][flag] to either
/// track the presence of, or count the number of times a command-line flag is specified.
///
/// This includes all built-in integer types and [`bool`], but also [`Option<bool>`], which will be
/// [`None`] if the flag isn't encountered at all, or [`Some`] if it is.
///
/// [flag]: crate::attrs::field::flag
///
/// # Example
///
/// Accept a `--verbose` flag that can be specified multiple times:
///
/// ```
/// use larpa::Command;
///
/// #[derive(Debug, PartialEq, Command)]
/// struct MyCmd {
///     #[larpa(flag, name = ["--verbose", "-v"])]
///     verbosity: u8,
/// }
///
/// assert_eq!(MyCmd::from_iter(["my-cmd"]), MyCmd { verbosity: 0 });
/// assert_eq!(MyCmd::from_iter(["my-cmd", "--verbose"]), MyCmd { verbosity: 1 });
/// assert_eq!(MyCmd::from_iter(["my-cmd", "-v"]), MyCmd { verbosity: 1 });
/// assert_eq!(MyCmd::from_iter(["my-cmd", "-vvv"]), MyCmd { verbosity: 3 });
/// ```
pub trait Flag: Default + Sealed {}

impl Flag for bool {}

// Enables three-value logic, eg. `--pager` and `--no-pager` with the default being "auto".
impl Flag for Option<bool> {}

macro_rules! impl_ints {
    ($($int:ty),*) => {
        $(
            impl Sealed for $int {}
            impl FlagInternal for $int {
                #[inline]
                fn set(&mut self, _cx: &Context) {
                    *self = self.saturating_add(1);
                }
            }
            impl InvertibleFlag for $int {
                #[inline]
                fn unset(&mut self, _cx: &Context) {
                    *self = self.saturating_sub(1);
                }
            }
            impl Flag for $int {}
        )*
    };
}

impl_ints!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
