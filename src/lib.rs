use std::alloc::{alloc, dealloc, Layout};
use std::borrow::{Borrow, BorrowMut};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ptr;

#[derive(Debug)]
pub struct DynArray<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> DynArray<T> {
    /// # Safety
    ///
    /// ptr has to point to an initialized array of type T and length len
    pub unsafe fn from_parts(ptr: *mut T, len: usize) -> Self {
        DynArray { ptr, len }
    }

    pub fn into_parts(self) -> (*mut T, usize) {
        let me = ManuallyDrop::new(self);
        (me.ptr, me.len)
    }

    /// allocate new uninit DynArray of size `len`
    pub fn new_uninit(len: usize) -> DynArray<MaybeUninit<T>> {
        let layout = Layout::array::<T>(len).unwrap();

        unsafe {
            let ptr = alloc(layout) as *mut MaybeUninit<T>;

            DynArray::<MaybeUninit<T>>::from_parts(ptr, len)
        }
    }

    /// allocate new DynArray of size `len` and fill with default value
    pub fn new(len: usize) -> Self
    where
        T: Default,
    {
        let mut dyn_array = Self::new_uninit(len);

        for elem in dyn_array.iter_mut() {
            elem.write(T::default());
        }

        dyn_array.assume_init()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[allow(clippy::should_implement_trait)]
    /// Like FromIterator, but only for ExactSizeIterator
    pub fn from_iter<I: ExactSizeIterator<Item = T>>(mut iter: I) -> Self {
        let mut dyn_array = Self::new_uninit(iter.len());

        for elem in dyn_array.iter_mut() {
            elem.write(iter.next().expect("Iterator provided false size hint"));
        }

        assert!(iter.next().is_none(), "Iterator provided false size hint");

        dyn_array.assume_init()
    }
}

impl<T> DynArray<MaybeUninit<T>> {
    pub fn assume_init(self) -> DynArray<T> {
        let (ptr, len) = self.into_parts();
        unsafe { DynArray::from_parts(ptr as *mut T, len) }
    }
}

impl<T> Drop for DynArray<T> {
    fn drop(&mut self) {
        println!("Dropping {:#?}", self.ptr);

        let layout = Layout::array::<T>(self.len).unwrap();

        unsafe {
            dealloc(self.ptr as *mut u8, layout);
        }
    }
}

/// Clone slice into new DynArray
impl<T: Clone> From<&[T]> for DynArray<T> {
    fn from(slice: &[T]) -> Self {
        let mut dyn_array = Self::new_uninit(slice.len());

        for i in 0..slice.len() {
            dyn_array[i].write(slice[i].clone());
        }

        dyn_array.assume_init()
    }
}

impl<T, const N: usize> From<[T; N]> for DynArray<T> {
    fn from(array: [T; N]) -> Self {
        let boxed_array: Box<[T]> = Box::new(array);
        DynArray::from(boxed_array)
    }
}

impl<T> From<Box<[T]>> for DynArray<T> {
    fn from(boxed_slice: Box<[T]>) -> Self {
        unsafe {
            let len = boxed_slice.len();
            let ptr = (*Box::into_raw(boxed_slice)).as_mut_ptr();

            Self::from_parts(ptr, len)
        }
    }
}

impl<T: Clone> From<&mut [T]> for DynArray<T> {
    fn from(slice: &mut [T]) -> Self {
        DynArray::from(slice as &[T])
    }
}

impl<T: Clone> Clone for DynArray<T> {
    fn clone(&self) -> Self {
        DynArray::from(&**self)
    }
}

impl<T> Deref for DynArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> DerefMut for DynArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> AsRef<[T]> for DynArray<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for DynArray<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> Borrow<[T]> for DynArray<T> {
    fn borrow(&self) -> &[T] {
        &self[..]
    }
}

impl<T> BorrowMut<[T]> for DynArray<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

pub struct IntoIter<T> {
    dyn_array: DynArray<ManuallyDrop<T>>,
    idx: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.dyn_array.len {
            return None;
        }

        self.idx += 1;

        unsafe {
            let ptr = self.dyn_array.ptr.add(self.idx - 1) as *mut T;
            Some(ptr::read(ptr))
        }
    }
}

impl<T> IntoIterator for DynArray<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            let (ptr, len) = self.into_parts();
            let dyn_array = DynArray::from_parts(ptr as *mut ManuallyDrop<T>, len);

            IntoIter { dyn_array, idx: 0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use super::DynArray;

    #[test]
    fn uninit_test() {
        let mut a: DynArray<MaybeUninit<u32>> = DynArray::new_uninit(20);

        for i in 0..20 {
            a[i].write(i as u32);
        }

        drop(a);
    }

    #[test]
    fn default_test() {
        let a: DynArray<u32> = DynArray::new(20);

        println!("Created");

        for i in a {
            println!("Loop");
            assert_eq!(i, 0);
        }

        println!("Loop done");
    }
}
