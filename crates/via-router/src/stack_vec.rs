use core::array;
use std::vec;

pub struct StackVec<T, const N: usize> {
    inner: StackVecInner<T, N>,
}

pub struct StackVecIntoIter<T, const N: usize> {
    iter: StackVecIntoIterInner<T, N>,
}

enum StackVecInner<T, const N: usize> {
    Stack {
        data: Option<[Option<T>; N]>,
        len: usize,
    },
    Heap {
        data: Vec<Option<T>>,
    },
}

enum StackVecIntoIterInner<T, const N: usize> {
    Stack(array::IntoIter<Option<T>, N>),
    Heap(vec::IntoIter<Option<T>>),
}

fn get<T: Copy, const N: usize>(data: &StackVecInner<T, N>, index: usize) -> Option<T> {
    match data {
        StackVecInner::Stack { data, .. } => *data.as_ref()?.get(index)?,
        StackVecInner::Heap { data } => *data.get(index)?,
    }
}

impl<T: Copy, const N: usize> StackVec<T, N> {
    /// Returns a option containing a reference to the element at `index`.
    ///
    pub fn new(init: [Option<T>; N]) -> Self {
        let len = init.iter().filter(|entry| entry.is_some()).count();

        Self {
            inner: StackVecInner::Stack {
                data: Some(init),
                len,
            },
        }
    }

    /// Returns an option containing a reference to the element at `index`.
    ///
    pub fn get(&self, index: usize) -> Option<T> {
        get(&self.inner, index)
    }

    /// Returns the number of elements in the vec.
    ///
    pub fn len(&self) -> usize {
        match &self.inner {
            StackVecInner::Stack { len, .. } => *len,
            StackVecInner::Heap { data } => data.len(),
        }
    }

    /// Appends an element to the end of the vec.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    pub fn push(&mut self, value: T) {
        let inner = &mut self.inner;

        loop {
            match inner {
                StackVecInner::Stack { data, len } => {
                    let array = match data {
                        Some(ptr) => ptr,
                        None => {
                            // Placeholder for tracing...
                            *inner = StackVecInner::Heap { data: Vec::new() };
                            continue;
                        }
                    };

                    // Copy and store the `len` pointer as `index`.
                    let index = *len;

                    // If `index` is less than `N`, it points to a vacant entry.
                    if index < N {
                        // Store `value` in the vacant entry.
                        array[index] = Some(value);
                        // Increment the `len` pointer.
                        *len = index + 1;
                        // Exit the loop.
                        break;
                    }

                    // The stack is full. We're going to move the data in `array`
                    // to the heap and transition the internal state of self to
                    // `StackVecData:Heap`.

                    // Allocate a new vec to store the data in `array`.
                    let mut vec = Vec::new();

                    // Move the array out of `store` and into `vec`.
                    vec.extend(data.take().into_iter().flatten());

                    // Transition `data` to `StackVecData::Heap`.
                    *inner = StackVecInner::Heap { data: vec };

                    // Continue to the next iteration to append `value` to `vec`.
                    // The iterative approach that we take here confirms that the
                    // internal state of `self` remains consistent.
                }
                StackVecInner::Heap { data } => {
                    // We're already on the heap. Append `value` to the vec.
                    data.push(Some(value));
                    // Exit the loop.
                    break;
                }
            }
        }
    }
}

impl<T: Copy, const N: usize> IntoIterator for StackVec<T, N> {
    type IntoIter = StackVecIntoIter<T, N>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        match self.inner {
            StackVecInner::Stack { data, .. } => {
                #[allow(clippy::unnecessary_lazy_evaluations)]
                let array = data.unwrap_or_else(|| [None; N]);
                let into_iter = array.into_iter();

                StackVecIntoIter {
                    iter: StackVecIntoIterInner::Stack(into_iter),
                }
            }
            StackVecInner::Heap { data: vec } => {
                let into_iter = vec.into_iter();

                StackVecIntoIter {
                    iter: StackVecIntoIterInner::Heap(into_iter),
                }
            }
        }
    }
}

impl<T, const N: usize> Iterator for StackVecIntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iter {
            StackVecIntoIterInner::Stack(iter) => iter.next()?,
            StackVecIntoIterInner::Heap(iter) => iter.next()?,
        }
    }
}
