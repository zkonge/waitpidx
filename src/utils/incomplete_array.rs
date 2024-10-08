#![allow(dead_code)]
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    marker::PhantomData,
    slice,
};

#[repr(C)]
#[derive(Default)]
pub(crate) struct IncompleteArray<T>(PhantomData<T>, [T; 0]);

impl<T> IncompleteArray<T> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData, [])
    }
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
    }
    #[inline]
    pub const unsafe fn as_slice(&self, len: usize) -> &[T] {
        slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}

impl<T> Debug for IncompleteArray<T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        fmt.debug_struct("IncompleteArray").finish_non_exhaustive()
    }
}
