use std::convert::Into;

/// The result of computing the index of a tree node's sibling.
pub enum SiblingIndex {
    /// Numerical underflow in computing the index.
    Underflow,
    /// Numerical overflow in computing the index.
    Overflow,
    /// The computed index is out of range, with the second value giving the
    /// number of siblings.
    OutOfRange(usize, usize),
    /// A successfully computed index value.
    Valid(usize),
}

impl SiblingIndex {
    pub fn of(sibling_count: usize,
              here_index: usize,
              offset: isize) -> Self {
        let offset_abs = offset.abs();
        if offset_abs < 0 {
            // offset is Int::min_value().
            let mut new_index = match here_index.checked_sub(1) {
                Some(x) => x,
                None => return SiblingIndex::Underflow,
            };
            new_index = match new_index.checked_sub((offset_abs + 1isize).abs() as usize) {
                Some(x) => x,
                None => return SiblingIndex::Underflow,
            };
            SiblingIndex::Valid(new_index)
        } else if offset_abs == 0 {
            SiblingIndex::Valid(here_index)
        } else {
            let new_index = match here_index.checked_add(offset_abs as usize) {
                Some(x) => x,
                None => return SiblingIndex::Overflow,
            };
            if new_index >= sibling_count {
                return SiblingIndex::OutOfRange(new_index, sibling_count);
            }
            SiblingIndex::Valid(new_index)
        }
    }

    /// Safely computes the index of a tree node's sibling.
    ///
    /// For `sibling_count` siblings and the current node at `here_index`, the
    /// index of the node that is the given offset from `here_index` is computed
    /// using checked arithmetic.
    pub fn compute(sibling_count: usize,
                   here_index: usize,
                   offset: isize) -> Option<usize> {
        SiblingIndex::of(sibling_count, here_index, offset).into()
    }
}

impl Into<Option<usize>> for SiblingIndex {
    /// Unwraps the index to get its value, or panics with an error message if
    /// `self` is not `SiblingIndex::Valid`.
    fn into(self) -> Option<usize> {
        match self {
            SiblingIndex::Underflow => panic!["numerical underflow computing sibling offset"],
            SiblingIndex::Overflow => panic!["numerical overflow computing sibling offset"],
            SiblingIndex::OutOfRange(_, _) => None,
            SiblingIndex::Valid(new_index) => Some(new_index),
        }
    }
}

/// The result of computing the index of a child.
pub enum ChildIndex {
    /// The computed index is out of range, with the second value giving the
    /// number of children.
    OutOfRange(usize, usize),
    /// A successfully computed index value.
    Valid(usize),
}

impl ChildIndex {
    /// Validates that a tree node has a child at the given index.
    pub fn of(child_count: usize, index: usize) -> Self {
        if index >= child_count {
            ChildIndex::OutOfRange(index, child_count)
        } else {
            ChildIndex::Valid(index)
        }
    }

    pub fn compute(child_count: usize, index: usize) -> Option<usize> {
        ChildIndex::of(child_count, index).into()
    }
}

impl Into<Option<usize>> for ChildIndex {
    /// Unwraps the index to get its value, or panics with an error message if
    /// `self` is not `ChildIndex::Valid`.    
    fn into(self) -> Option<usize> {
        match self {
            ChildIndex::OutOfRange(_, _) => None,
            ChildIndex::Valid(new_index) => Some(new_index),
        }
     }
}
