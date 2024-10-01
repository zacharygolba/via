use core::{array, mem};
use std::vec;

pub struct StackVec<T, const N: usize> {
    data: StackVecData<T, N>,
}

pub struct StackVecIntoIter<T, const N: usize> {
    iter: StackVecIntoIterInner<T, N>,
}

enum StackVecData<T, const N: usize> {
    Stack { array: [Option<T>; N], len: usize },
    Heap { vec: Vec<Option<T>> },
}

enum StackVecIntoIterInner<T, const N: usize> {
    Stack(array::IntoIter<Option<T>, N>),
    Heap(vec::IntoIter<Option<T>>),
}

fn get<T, const N: usize>(data: &StackVecData<T, N>, index: usize) -> Option<&T> {
    match data {
        StackVecData::Stack { array, .. } => array.get(index)?.as_ref(),
        StackVecData::Heap { vec } => vec.get(index)?.as_ref(),
    }
}

impl<T: Copy, const N: usize> StackVec<T, N> {
    /// Returns a option containing a reference to the element at `index`.
    ///
    pub fn new() -> Self {
        Self {
            data: StackVecData::Stack {
                array: [None; N],
                len: 0,
            },
        }
    }

    /// Returns an option containing a reference to the element at `index`.
    ///
    pub fn get(&self, index: usize) -> Option<&T> {
        get(&self.data, index)
    }

    /// Returns the number of elements in the vec.
    ///
    pub fn len(&self) -> usize {
        match &self.data {
            StackVecData::Stack { len, .. } => *len,
            StackVecData::Heap { vec } => vec.len(),
        }
    }

    /// Appends an element to the end of the vec.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    pub fn push(&mut self, value: T) {
        let data = &mut self.data;

        loop {
            match data {
                StackVecData::Stack { array, len } => {
                    // Copy and store the `len` pointer so it can be used for
                    let index = *len;

                    // If `index` is less than `N`, we can store `value` at `ptr`.
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

                    // Move the data from the stack-allocated `array` to the
                    // heap-allocated vec. Temporarily replace `array` with
                    // `[None; N]` to avoid dropping the contents of the array.
                    vec.extend(mem::replace(array, [None; N]));

                    // Transition the internal state of self at `data` to the
                    // Heap variant.
                    *data = StackVecData::Heap { vec };

                    // Continue to the next iteration to push `value` to the
                    // heap. This allows us to confirm that the internal state
                    // transition was successful.
                }
                StackVecData::Heap { vec } => {
                    // We're already on the heap. Append `value` to the vec.
                    vec.push(Some(value));
                    // Exit the loop.
                    break;
                }
            }
        }
    }
}

impl<T, const N: usize> IntoIterator for StackVec<T, N> {
    type IntoIter = StackVecIntoIter<T, N>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        match self.data {
            StackVecData::Stack { array, .. } => StackVecIntoIter {
                iter: StackVecIntoIterInner::Stack(array.into_iter()),
            },
            StackVecData::Heap { vec } => StackVecIntoIter {
                iter: StackVecIntoIterInner::Heap(vec.into_iter()),
            },
        }
    }
}

impl<T, const N: usize> Iterator for StackVecIntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.iter {
            StackVecIntoIterInner::Stack(stack) => stack.next()?,
            StackVecIntoIterInner::Heap(heap) => heap.next()?,
        }
    }
}
