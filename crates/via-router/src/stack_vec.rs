use core::{array, slice};
use std::vec;

pub struct StackVec<T, const N: usize> {
    data: StackVecData<T, N>,
}

pub struct StackVecIter<'a, T> {
    iter: slice::Iter<'a, Option<T>>,
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
    pub fn new() -> Self {
        Self {
            data: StackVecData::Stack {
                array: [None; N],
                len: 0,
            },
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        get(&self.data, index)
    }

    pub fn len(&self) -> usize {
        match &self.data {
            StackVecData::Stack { len, .. } => *len,
            StackVecData::Heap { vec } => vec.len(),
        }
    }

    pub fn iter(&self) -> StackVecIter<T> {
        let iter = match self.data {
            StackVecData::Stack { ref array, len } => array[..len].iter(),
            StackVecData::Heap { ref vec } => vec.iter(),
        };

        StackVecIter { iter }
    }

    pub fn push(&mut self, value: T) {
        let data = &mut self.data;

        match data {
            // Attempt to store `value` on the stack. If there is no vacant
            // entry, move the data to the heap.
            StackVecData::Stack { array, len } => {
                let index = *len;

                if index < N {
                    array[index] = Some(value);
                    *len = index + 1;
                } else {
                    let mut vec = array.to_vec();

                    vec.push(Some(value));
                    *data = StackVecData::Heap { vec };
                }
            }
            // We have a heap-allocated vec. Push `value` into it.
            StackVecData::Heap { vec } => {
                vec.push(Some(value));
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

impl<'a, T> Iterator for StackVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next()? {
            Some(value) => Some(value),
            None => None,
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
