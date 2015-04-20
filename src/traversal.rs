pub trait Queue<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn first(&self) -> Option<&T>;
    fn shift(&mut self, t: T);
    fn unshift(&mut self) -> Option<T>;
}

pub struct DepthQueue<T> {
    v: Vec<T>,
}

impl<T> Queue<T> for DepthQueue<T> {
    fn len(&self) -> usize { self.v.len() }
    fn first(&self) -> Option<&T> { self.v.first() }
    fn shift(&mut self, t: T) { self.v.push(t); }
    fn unshift(&mut self) -> Option<T> { self.v.pop() }
}
