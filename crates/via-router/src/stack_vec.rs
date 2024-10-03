use std::mem;

pub struct StackVec<T, const N: usize> {
    data: StackVecData<T, N>,
}

enum StackVecData<T, const N: usize> {
    Stack { array: [Option<T>; N], len: usize },
    Heap { vec: Vec<Option<T>> },
}

fn get<T: Copy, const N: usize>(data: &StackVecData<T, N>, index: usize) -> Option<T> {
    match data {
        StackVecData::Stack { array, .. } => array.get(index).copied()?,
        StackVecData::Heap { vec } => vec.get(index).copied()?,
    }
}

impl<T: Copy, const N: usize> StackVec<T, N> {
    pub fn new(init: [Option<T>; N]) -> Self {
        Self {
            data: StackVecData::Stack {
                array: init,
                len: 0,
            },
        }
    }

    /// Returns a option containing a copy of the element at `index`.
    ///
    pub fn get(&self, index: usize) -> Option<T> {
        get(&self.data, index)
    }

    /// Returns the number of elements in self.
    ///
    pub fn len(&self) -> usize {
        match &self.data {
            StackVecData::Stack { len, .. } => *len,
            StackVecData::Heap { vec } => vec.len(),
        }
    }

    /// Returns the number of elements in self.
    ///
    pub fn is_empty(&self) -> bool {
        match &self.data {
            StackVecData::Stack { len, .. } => *len == 0,
            StackVecData::Heap { vec } => vec.is_empty(),
        }
    }

    /// Appends an element to the end of self.
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
                    // Copy and store the `len` pointer as `index`.
                    let index = *len;

                    // If `index` is less than `N`, it points to a vacant entry.
                    if index < N {
                        // Store `value` in the vacant entry.
                        array[index] = Some(value);
                        // Increment the `len` pointer.
                        *len += 1;
                        // Exit the loop
                        break;
                    }

                    // The stack is full. We're going to move the data in `array`
                    // to the heap and transition the internal state of self to
                    // `StackVecData:Heap`.

                    // Allocate a new vec to store the data in `array`.
                    let mut vec = Vec::with_capacity(N + 1);

                    // Move the data from `array` to `vec`.
                    vec.extend(mem::replace(array, [None; N]));

                    // Transition `data` to `StackVecData::Heap`.
                    drop(mem::replace(data, StackVecData::Heap { vec }));

                    // Continue to the next iteration to append `value` to `vec`.
                    // The iterative approach that we take here confirms that the
                    // internal state of `self` remains consistent.
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
