use std::ffi::{CString, NulError};
use std::str::Utf8Error;
use thiserror::Error;

macro_rules! impl_c_repr_of_for {
    ($typ:ty) => {
        impl CReprOf<$typ> for $typ {
            fn c_repr_of(input: $typ) -> Result<$typ, CReprOfError> {
                Ok(input)
            }
        }
    };

    ($from_typ:ty, $to_typ:ty) => {
        impl CReprOf<$from_typ> for $to_typ {
            fn c_repr_of(input: $from_typ) -> Result<$to_typ, CReprOfError> {
                Ok(input as $to_typ)
            }
        }
    };
}

/// implements a noop implementation of the CDrop trait for a given type.
macro_rules! impl_c_drop_for {
    ($typ:ty) => {
        impl CDrop for $typ {
            fn do_drop(&mut self) -> Result<(), CDropError> {
                Ok(())
            }
        }
    };
}

macro_rules! impl_as_rust_for {
    ($typ:ty) => {
        impl AsRust<$typ> for $typ {
            fn as_rust(&self) -> Result<$typ, AsRustError> {
                Ok(*self)
            }
        }
    };

    ($from_typ:ty, $to_typ:ty) => {
        impl AsRust<$to_typ> for $from_typ {
            fn as_rust(&self) -> Result<$to_typ, AsRustError> {
                Ok(*self as $to_typ)
            }
        }
    };
}

macro_rules! impl_rawpointerconverter_for {
    ($typ:ty) => {
        impl RawPointerConverter<$typ> for $typ {
            fn into_raw_pointer(self) -> *const $typ {
                convert_into_raw_pointer(self)
            }
            fn into_raw_pointer_mut(self) -> *mut $typ {
                convert_into_raw_pointer_mut(self)
            }
            unsafe fn from_raw_pointer(
                input: *const $typ,
            ) -> Result<Self, UnexpectedNullPointerError> {
                take_back_from_raw_pointer(input)
            }
            unsafe fn from_raw_pointer_mut(
                input: *mut $typ,
            ) -> Result<Self, UnexpectedNullPointerError> {
                take_back_from_raw_pointer_mut(input)
            }
        }
    };
}

pub fn point_to_string(
    pointer: *mut *const libc::c_char,
    string: String,
) -> Result<(), CReprOfError> {
    unsafe { *pointer = std::ffi::CString::c_repr_of(string)?.into_raw_pointer() }
    Ok(())
}

#[derive(Error, Debug)]
pub enum CReprOfError {
    #[error("A string contains a nul bit")]
    StringContainsNullBit(#[from] NulError),
    #[error("An error occurred during conversion to C repr; {}", .0)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Trait showing that the struct implementing it is a `repr(C)` compatible view of the parametrized
/// type that can be created from an object of this type.
pub trait CReprOf<T>: Sized + CDrop {
    fn c_repr_of(input: T) -> Result<Self, CReprOfError>;
}

#[derive(Error, Debug)]
pub enum CDropError {
    #[error("unexpected null pointer")]
    NullPointer(#[from] UnexpectedNullPointerError),
    #[error("An error occurred while dropping C struct: {}", .0)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Trait showing that the C-like struct implementing it can free up its part of memory that are not
/// managed by Rust.
pub trait CDrop {
    fn do_drop(&mut self) -> Result<(), CDropError>;
}

#[derive(Error, Debug)]
pub enum AsRustError {
    #[error("unexpected null pointer")]
    NullPointer(#[from] UnexpectedNullPointerError),

    #[error("could not convert string as it is not UTF-8: {}", .0)]
    Utf8Error(#[from] Utf8Error),
    #[error("An error occurred during conversion to Rust: {}", .0)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Trait showing that the struct implementing it is a `repr(C)` compatible view of the parametrized
/// type and that an instance of the parametrized type can be created form this struct
pub trait AsRust<T> {
    fn as_rust(&self) -> Result<T, AsRustError>;
}

#[derive(Error, Debug)]
#[error("Could not use raw pointer: unexpected null pointer")]
pub struct UnexpectedNullPointerError;

/// Trait representing the creation of a raw pointer from a struct and the recovery of said pointer.
///
/// The `from_raw_pointer` function should be used only on pointers obtained through the
/// `into_raw_pointer` method (and is thus unsafe as we don't have any way to get insurance of that
/// from the compiler).
///
/// The `from_raw_pointer` effectively takes back ownership of the pointer. If you didn't create the
/// pointer yourself, please use the `as_ref` method on the raw pointer to borrow it
pub trait RawPointerConverter<T>: Sized {
    fn into_raw_pointer(self) -> *const T;
    fn into_raw_pointer_mut(self) -> *mut T;
    unsafe fn from_raw_pointer(input: *const T) -> Result<Self, UnexpectedNullPointerError>;
    unsafe fn from_raw_pointer_mut(input: *mut T) -> Result<Self, UnexpectedNullPointerError>;

    unsafe fn drop_raw_pointer(input: *const T) -> Result<(), UnexpectedNullPointerError> {
        Self::from_raw_pointer(input).map(|_| ())
    }

    unsafe fn drop_raw_pointer_mut(input: *mut T) -> Result<(), UnexpectedNullPointerError> {
        Self::from_raw_pointer_mut(input).map(|_| ())
    }
}

#[doc(hidden)]
pub fn convert_into_raw_pointer<T>(pointee: T) -> *const T {
    Box::into_raw(Box::new(pointee)) as _
}

#[doc(hidden)]
pub fn convert_into_raw_pointer_mut<T>(pointee: T) -> *mut T {
    Box::into_raw(Box::new(pointee))
}

#[doc(hidden)]
pub unsafe fn take_back_from_raw_pointer<T>(
    input: *const T,
) -> Result<T, UnexpectedNullPointerError> {
    take_back_from_raw_pointer_mut(input as _)
}

#[doc(hidden)]
pub unsafe fn take_back_from_raw_pointer_mut<T>(
    input: *mut T,
) -> Result<T, UnexpectedNullPointerError> {
    if input.is_null() {
        Err(UnexpectedNullPointerError)
    } else {
        Ok(*Box::from_raw(input))
    }
}

/// Trait to create borrowed references to type T, from a raw pointer to a T
pub trait RawBorrow<T> {
    unsafe fn raw_borrow<'a>(input: *const T) -> Result<&'a Self, UnexpectedNullPointerError>;
}

/// Trait to create mutable borrowed references to type T, from a raw pointer to a T
pub trait RawBorrowMut<T> {
    unsafe fn raw_borrow_mut<'a>(input: *mut T)
        -> Result<&'a mut Self, UnexpectedNullPointerError>;
}

/// Trait that allows obtaining a borrowed reference to a type T from a raw pointer to T
impl<T> RawBorrow<T> for T {
    unsafe fn raw_borrow<'a>(input: *const T) -> Result<&'a Self, UnexpectedNullPointerError> {
        input.as_ref().ok_or_else(|| UnexpectedNullPointerError)
    }
}

/// Trait that allows obtaining a mutable borrowed reference to a type T from a raw pointer to T
impl<T> RawBorrowMut<T> for T {
    unsafe fn raw_borrow_mut<'a>(
        input: *mut T,
    ) -> Result<&'a mut Self, UnexpectedNullPointerError> {
        input.as_mut().ok_or_else(|| UnexpectedNullPointerError)
    }
}

impl RawPointerConverter<libc::c_void> for std::ffi::CString {
    fn into_raw_pointer(self) -> *const libc::c_void {
        self.into_raw() as _
    }

    fn into_raw_pointer_mut(self) -> *mut libc::c_void {
        self.into_raw() as _
    }

    unsafe fn from_raw_pointer(
        input: *const libc::c_void,
    ) -> Result<Self, UnexpectedNullPointerError> {
        Self::from_raw_pointer_mut(input as *mut libc::c_void)
    }

    unsafe fn from_raw_pointer_mut(
        input: *mut libc::c_void,
    ) -> Result<Self, UnexpectedNullPointerError> {
        if input.is_null() {
            Err(UnexpectedNullPointerError)
        } else {
            Ok(std::ffi::CString::from_raw(input as *mut libc::c_char))
        }
    }
}

impl RawPointerConverter<libc::c_char> for std::ffi::CString {
    fn into_raw_pointer(self) -> *const libc::c_char {
        self.into_raw() as _
    }

    fn into_raw_pointer_mut(self) -> *mut libc::c_char {
        self.into_raw()
    }

    unsafe fn from_raw_pointer(
        input: *const libc::c_char,
    ) -> Result<Self, UnexpectedNullPointerError> {
        Self::from_raw_pointer_mut(input as *mut libc::c_char)
    }

    unsafe fn from_raw_pointer_mut(
        input: *mut libc::c_char,
    ) -> Result<Self, UnexpectedNullPointerError> {
        if input.is_null() {
            Err(UnexpectedNullPointerError)
        } else {
            Ok(std::ffi::CString::from_raw(input as *mut libc::c_char))
        }
    }
}

impl RawBorrow<libc::c_char> for std::ffi::CStr {
    unsafe fn raw_borrow<'a>(
        input: *const libc::c_char,
    ) -> Result<&'a Self, UnexpectedNullPointerError> {
        if input.is_null() {
            Err(UnexpectedNullPointerError)
        } else {
            Ok(Self::from_ptr(input))
        }
    }
}

impl_c_drop_for!(usize);
impl_c_drop_for!(u8);
impl_c_drop_for!(i16);
impl_c_drop_for!(u16);
impl_c_drop_for!(i32);
impl_c_drop_for!(u32);
impl_c_drop_for!(i64);
impl_c_drop_for!(u64);
impl_c_drop_for!(f32);
impl_c_drop_for!(f64);
impl_c_drop_for!(bool);
impl_c_drop_for!(std::ffi::CString);

impl_c_repr_of_for!(usize);
impl_c_repr_of_for!(i16);
impl_c_repr_of_for!(u16);
impl_c_repr_of_for!(i32);
impl_c_repr_of_for!(u32);
impl_c_repr_of_for!(i64);
impl_c_repr_of_for!(u64);
impl_c_repr_of_for!(f32);
impl_c_repr_of_for!(f64);
impl_c_repr_of_for!(bool);

impl_c_repr_of_for!(usize, i32);

impl CReprOf<bool> for u8 {
    fn c_repr_of(input: bool) -> Result<u8, CReprOfError> {
        Ok(if input { 1 } else { 0 })
    }
}

impl CReprOf<String> for std::ffi::CString {
    fn c_repr_of(input: String) -> Result<Self, CReprOfError> {
        Ok(std::ffi::CString::new(input)?)
    }
}

impl_as_rust_for!(usize);
impl_as_rust_for!(i16);
impl_as_rust_for!(u16);
impl_as_rust_for!(i32);
impl_as_rust_for!(u32);
impl_as_rust_for!(i64);
impl_as_rust_for!(u64);
impl_as_rust_for!(f32);
impl_as_rust_for!(f64);
impl_as_rust_for!(bool);

impl_as_rust_for!(i32, usize);

impl AsRust<bool> for u8 {
    fn as_rust(&self) -> Result<bool, AsRustError> {
        Ok((*self) != 0)
    }
}

impl AsRust<String> for std::ffi::CStr {
    fn as_rust(&self) -> Result<String, AsRustError> {
        self.to_str().map(|s| s.to_owned()).map_err(|e| e.into())
    }
}

impl_rawpointerconverter_for!(usize);
impl_rawpointerconverter_for!(i16);
impl_rawpointerconverter_for!(u16);
impl_rawpointerconverter_for!(i32);
impl_rawpointerconverter_for!(u32);
impl_rawpointerconverter_for!(i64);
impl_rawpointerconverter_for!(u64);
impl_rawpointerconverter_for!(f32);
impl_rawpointerconverter_for!(f64);
impl_rawpointerconverter_for!(bool);
