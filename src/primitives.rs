use std::sync::{Arc, RwLock};
use subject::{self, Source, Mapper, WrapListener, Subject, Receiver};


pub trait Event<A> {
    fn map<B, F>(&self, f: F) -> Map<A, B, F>
        where F: Fn(A) -> B;
    fn filter<F>(&self, f: F) -> Filter<A, F>
        where F: Fn(&A) -> bool + Send + Sync;
    fn iter(&self) -> Iter<A>;
}


pub struct Sink<A> {
    source: RwLock<Source<A>>,
}

impl<A: Send> Sink<A> {
    pub fn new() -> Sink<A> {
        Sink { source: RwLock::new(Source::new()) }
    }
}

impl<A: Send + Sync + Clone> Sink<A> {
    pub fn send(&self, a: A) {
        self.source.write().unwrap().send(a);
    }
}

impl<A: Send + Sync + Clone> Event<A> for Sink<A> {
    fn map<B, F>(&self, f: F) -> Map<A, B, F>
        where B: Send + Sync + Clone,
              F: Fn(A) -> B + Send + Sync,
    {
        Map::new(&mut *self.source.write().unwrap(), f)
    }

    fn filter<F>(&self, f: F) -> Filter<A, F>
        where F: Fn(&A) -> bool + Send + Sync,
    {
        Filter::new(&mut *self.source.write().unwrap(), f)
    }

    fn iter(&self) -> Iter<A> {
        Iter::new(&mut *self.source.write().unwrap())
    }
}


pub struct Map<A, B, F> {
    mapper: Arc<RwLock<Mapper<A, B, F>>>,
}

impl<A, B, F> Map<A, B, F>
    where A: Send + Sync + Clone,
          B: Send + Sync + Clone,
          F: Fn(A) -> B + Send + Sync,
{
    fn new<S: Subject<A>>(sub: &mut S, f: F) -> Map<A, B, F> {
        let map = Map { mapper: Arc::new(RwLock::new(Mapper::new(f))) };
        sub.listen(map.mapper.wrap());
        map
    }
}

impl<A, B, F> Event<B> for Map<A, B, F>
    where A: Send + Sync + Clone,
          B: Send + Sync + Clone,
          F: Fn(A) -> B + Send + Sync,
{
    fn map<C, G>(&self, g: G) -> Map<B, C, G>
        where C: Send + Sync + Clone,
              G: Fn(B) -> C + Send + Sync,
    {
        Map::new(&mut *self.mapper.write().unwrap(), g)
    }

    fn filter<G>(&self, g: G) -> Filter<B, G>
        where G: Fn(&B) -> bool + Send + Sync,
    {
        Filter::new(&mut *self.mapper.write().unwrap(), g)
    }

    fn iter(&self) -> Iter<B> {
        Iter::new(&mut *self.mapper.write().unwrap())
    }
}


pub struct Filter<A, F> {
    filter: Arc<RwLock<subject::Filter<A, F>>>,
}

impl<A, F> Filter<A, F>
    where A: Send + Sync + Clone,
          F: Fn(&A) -> bool + Send + Sync,
{
    fn new<S: Subject<A>>(sub: &mut S, f: F) -> Filter<A, F> {
        let filter = Filter {
            filter: Arc::new(RwLock::new(subject::Filter::new(f)))
        };
        sub.listen(filter.filter.wrap());
        filter
    }
}

impl<A, F> Event<A> for Filter<A, F>
    where A: Send + Sync + Clone,
          F: Fn(&A) -> bool + Send + Sync,
{
    fn map<B, G>(&self, g: G) -> Map<A, B, G>
        where B: Send + Sync + Clone,
              G: Fn(A) -> B + Send + Sync,
    {
        Map::new(&mut *self.filter.write().unwrap(), g)
    }

    fn filter<G>(&self, g: G) -> Filter<A, G>
        where G: Fn(&A) -> bool + Send + Sync,
    {
        Filter::new(&mut *self.filter.write().unwrap(), g)
    }

    fn iter(&self) -> Iter<A> {
        Iter::new(&mut *self.filter.write().unwrap())
    }
}


pub struct Iter<A> {
    recv: Arc<RwLock<Receiver<A>>>,
}

impl<A: Send + Sync + Clone> Iter<A> {
    fn new<S: Subject<A>>(sub: &mut S) -> Iter<A> {
        let iter = Iter { recv: Arc::new(RwLock::new(Receiver::new())) };
        sub.listen(iter.recv.wrap());
        iter
    }
}

impl<A: Send + Sync> Iterator for Iter<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        self.recv.write().unwrap().next()
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sink() {
        let sink = Sink::new();
        let mut iter = sink.iter();
        sink.send(1);
        sink.send(2);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
    }

    #[test]
    fn map() {
        let sink = Sink::new();
        let triple = sink.map(|x| 3 * x);
        let mut iter = triple.iter();
        sink.send(1);
        assert_eq!(iter.next(), Some(3));
    }

    #[test]
    fn filter() {
        let sink = Sink::new();
        let small = sink.filter(|&x: &i32| x < 11);
        let mut iter = small.iter();
        sink.send(12);
        sink.send(9);
        assert_eq!(iter.next(), Some(9));
    }
}