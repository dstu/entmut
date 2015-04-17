// Return code for seeking a sibling.
pub enum SiblingIndex {
    Root,
    Underflow,
    Overflow,
    OutOfRange(usize, usize),
    Valid(usize),
}

impl SiblingIndex {
    pub fn compute(sibling_count: usize,
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
    
    pub fn unwrap(&self) -> usize {
        match self {
            &SiblingIndex::Root =>
                panic!["root vertex has no siblings"],
            &SiblingIndex::Underflow =>
                panic!["numerical underflow computing sibling offset"],
            &SiblingIndex::Overflow =>
                panic!["numerical overflow computing sibling offset"],
            &SiblingIndex::OutOfRange(new_index, siblings) =>
                panic!["cannot address sibling at index {} (only {} siblings)", new_index, siblings],
            &SiblingIndex::Valid(new_index) =>
                new_index,
        }
    }
}

pub enum ChildIndex {
    OutOfRange(usize, usize),
    Valid(usize),
}

impl ChildIndex {
    pub fn compute(child_count: usize, index: usize) -> Self {
        if index >= child_count {
            ChildIndex::OutOfRange(index, child_count)
        } else {
            ChildIndex::Valid(index)
        }
    }

    pub fn unwrap(&self) -> usize {
        match self {
            &ChildIndex::OutOfRange(new_index, children) =>
                panic!["cannot address child at index {} (only {} children)", new_index, children],
            &ChildIndex::Valid(new_index) => new_index,
        }
     }
}
