use std::{array, vec};

pub struct StackVec<T, const N: usize> {
    data: StackVecData<T, N>,
}

pub struct StackVecIter<'a, T, const N: usize> {
    index: usize,
    data: &'a StackVecData<T, N>,
}

pub struct StackVecIntoIter<T, const N: usize> {
    inner: StackVecIntoIterInner<T, N>,
}

enum StackVecData<T, const N: usize> {
    Stack([Option<T>; N]),
    Heap(Vec<T>),
}

enum StackVecIntoIterInner<T, const N: usize> {
    Stack(array::IntoIter<Option<T>, N>),
    Heap(vec::IntoIter<T>),
}

fn get<T, const N: usize>(data: &StackVecData<T, N>, index: usize) -> Option<&T> {
    match data {
        StackVecData::Stack(stack) => stack.get(index)?.as_ref(),
        StackVecData::Heap(heap) => heap.get(index),
    }
}

impl<T: Copy, const N: usize> StackVec<T, N> {
    pub fn new() -> Self {
        Self {
            data: StackVecData::Stack([None; N]),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        get(&self.data, index)
    }

    pub fn len(&self) -> usize {
        match &self.data {
            StackVecData::Stack(stack) => stack.iter().flatten().count(),
            StackVecData::Heap(heap) => heap.len(),
        }
    }

    pub fn iter(&self) -> StackVecIter<T, N> {
        StackVecIter {
            index: 0,
            data: &self.data,
        }
    }

    pub fn push(&mut self, value: T) {
        let data = &mut self.data;

        loop {
            match data {
                // Attempt to store `value` on the stack. If there is no vacant
                // entry, move the data to the heap.
                StackVecData::Stack(stack) => {
                    if let Some(index) = stack.iter().position(Option::is_none) {
                        stack[index] = Some(value);
                        break;
                    }

                    let mut heap = Vec::new();

                    for option in std::mem::replace(stack, [None; N]) {
                        if let Some(item) = option {
                            heap.push(item);
                        }
                    }

                    *data = StackVecData::Heap(heap);
                }

                // We have a heap-allocated vector. Push `value` into it.
                StackVecData::Heap(heap) => {
                    heap.push(value);
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
        match self.data {
            StackVecData::Stack(stack) => StackVecIntoIter {
                inner: StackVecIntoIterInner::Stack(stack.into_iter()),
            },
            StackVecData::Heap(heap) => StackVecIntoIter {
                inner: StackVecIntoIterInner::Heap(heap.into_iter()),
            },
        }
    }
}

impl<'a, T: Copy, const N: usize> Iterator for StackVecIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let next = get(self.data, self.index)?;

        self.index += 1;
        Some(next)
    }
}

impl<T: Copy, const N: usize> Iterator for StackVecIntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            StackVecIntoIterInner::Stack(stack) => stack.next()?,
            StackVecIntoIterInner::Heap(heap) => heap.next(),
        }
    }
}

impl<const N: usize, T: Copy + std::fmt::Debug> DoubleEndedIterator for StackVecIntoIter<T, N> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let iter = match &mut self.inner {
            StackVecIntoIterInner::Stack(stack) => stack,
            StackVecIntoIterInner::Heap(heap) => {
                return heap.next_back();
            }
        };

        if let Some(next @ Some(_)) = iter.next_back() {
            return next;
        }

        for _ in 0..N {
            if let Some(next @ Some(_)) = iter.next_back() {
                return next;
            }
        }

        None
    }
}
