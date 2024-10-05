pub struct StackVec<T, const N: usize> {
    data: StackVecData<T, N>,
}

enum StackVecData<T, const N: usize> {
    Stack { array: [Option<T>; N], len: usize },
    Heap { vec: Vec<Option<T>> },
}

impl<T, const N: usize> StackVec<T, N> {
    pub fn new(init: [Option<T>; N]) -> Self {
        if cfg!(debug_assertions) {
            for option in &init {
                assert!(option.is_none());
            }
        }

        Self {
            data: StackVecData::Stack {
                array: init,
                len: 0,
            },
        }
    }

    /// Returns a option containing a copy of the element at `index`.
    ///
    pub fn get(&self, index: usize) -> Option<&T> {
        match &self.data {
            StackVecData::Stack { array, .. } => array.get(index).and_then(Option::as_ref),
            StackVecData::Heap { vec } => vec.get(index).and_then(Option::as_ref),
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
        let vec = loop {
            match data {
                StackVecData::Heap { vec } => break vec,
                StackVecData::Stack { array, len } => {
                    // Copy and store the `len` pointer as `index`.
                    let index = *len;

                    // If `index` is less than `N`, it points to a vacant entry.
                    if index < N {
                        // Store `value` in the vacant entry.
                        array[index] = Some(value);
                        // Increment the `len` pointer.
                        *len = index + 1;
                        // Exit the loop.
                        return;
                    }

                    // The stack is full. We're going to move the data in `array`
                    // to the heap and transition the internal state of self to
                    // `StackVecData:Heap`.

                    // Allocate a new vec to store the data in `array`.
                    let mut vec = Vec::with_capacity(N + 1);

                    for option in array {
                        // Move the option from `array` to `vec`.
                        vec.push(option.take());
                    }

                    // Transition `data` to `StackVecData::Heap`.
                    *data = StackVecData::Heap { vec };

                    // Continue to the next iteration to append `value` to `vec`.
                    // The iterative approach that we take here confirms that the
                    // internal state of `self` remains consistent.
                }
            }
        };

        // Append `value` to `vec`.
        vec.push(Some(value));
    }
}
