use std::{
    alloc,
    mem::{align_of, size_of},
    slice,
    marker::PhantomData
};

pub struct DoubleTypeBuffer<T, U> {
    buffer: *mut u8,
    layout: alloc::Layout,
    phantom_t: PhantomData<T>,
    phantom_u: PhantomData<U>
}

impl<T, U> DoubleTypeBuffer<T, U> {
    pub fn with_lengths<V, W>(length_v: usize, length_w: usize) -> Self {
        let alignment_v = align_of::<V>();
        let alignment_w = align_of::<W>();
        let alignment = std::cmp::max(alignment_v, alignment_w);

        let size_v = Self::size_of_aligned::<V>();
        let size_w = Self::size_of_aligned::<W>();

        let size = std::cmp::max(size_v * length_v, size_w * length_w);

        unsafe {
            let layout = alloc::Layout::from_size_align_unchecked(size, alignment);
            let buffer = alloc::alloc(layout);

            Self {
                buffer, layout,
                phantom_t: PhantomData,
                phantom_u: PhantomData
            }
        }
    }

    fn size_of_aligned<V>() -> usize {
        Self::complement_to_multiple(size_of::<V>(), align_of::<V>())
    }

    fn complement_to_multiple(a: usize, b: usize) -> usize {
        if a % b == 0  {
            a
        }
        else {
            a + b - a % b
        }
    }

    pub fn as_slice_first(&self) -> &[T] {
        self.as_slice::<T>()
    }

    pub fn as_mut_slice_first(&mut self) -> &mut [T] {
        self.as_mut_slice::<T>()
    }

    pub fn as_slice_second(&self) -> &[U] {
        self.as_slice::<U>()
    }

    pub fn as_mut_slice_second(&mut self) -> &mut [U] {
        self.as_mut_slice::<U>()
    }

    fn as_slice<V>(&self) -> &[V] {
        let buffer = self.buffer as *const V;
        let length = self.layout.size() / Self::size_of_aligned::<V>();
        unsafe {
            slice::from_raw_parts(buffer, length)
        }
    }

    fn as_mut_slice<V>(&self) -> &mut [V] {
        let buffer = self.buffer as *mut V;
        let length = self.layout.size() / Self::size_of_aligned::<V>();
        unsafe {
            slice::from_raw_parts_mut(buffer, length)
        }
    }
}

impl<T, U> Drop for DoubleTypeBuffer<T, U> {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.buffer, self.layout);
        }
    }
}
