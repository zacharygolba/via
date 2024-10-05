pub struct StackVec<T, const N: usize> {
    inner: StackVecInner<T, N>,
}

enum StackVecInner<T, const N: usize> {
    Stack { data: [Option<T>; N], len: usize },
    Heap { data: Vec<Option<T>> },
}

impl<T, const N: usize> StackVec<T, N> {
    pub fn new(init: [Option<T>; N]) -> Self {
        if cfg!(debug_assertions) {
            for i in 0..N {
                assert!(init[i].is_none());
            }
        }

        Self {
            inner: StackVecInner::Stack { data: init, len: 0 },
        }
    }

    /// Returns a option containing a copy of the element at `index`.
    ///
    pub fn get(&self, index: usize) -> Option<&T> {
        match &self.inner {
            StackVecInner::Stack { data, .. } => data.get(index).and_then(Option::as_ref),
            StackVecInner::Heap { data } => data.get(index).and_then(Option::as_ref),
        }
    }

    /// Appends an element to the end of self.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    pub fn push(&mut self, value: T) {
        let vec = loop {
            match &mut self.inner {
                StackVecInner::Heap { data } => break data,
                StackVecInner::Stack { data, len } => {
                    // Copy and store the `len` pointer as `index`.
                    let index = *len;

                    // If `index` is less than `N`, it points to a vacant entry.
                    if index < N {
                        // Store `value` in the vacant entry.
                        data[index] = Some(value);
                        // Increment the `len` pointer.
                        *len = index + 1;
                        // Exit the loop.
                        return;
                    }

                    // The stack is full. We're going to move the data in `array`
                    // to the heap and transition the internal state of self to
                    // `StackVecData:Heap`.

                    // Allocate a new vec to store the data in `array`.
                    let mut vec = Vec::new();

                    vec.reserve(N + 1);

                    for i in 0..N {
                        // Move the data in `array` to `vec`.
                        vec.push(data[i].take());
                    }

                    // Transition `data` to `StackVecData::Heap`.
                    self.inner = StackVecInner::Heap { data: vec };

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
