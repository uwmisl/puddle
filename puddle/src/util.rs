
#[allow(dead_code)]
pub struct PairIter<I: Iterator>
    where I::Item: Copy
{
    it: I,
    last: Option<I::Item>,
}

impl<I: Iterator> Iterator for PairIter<I>
    where I::Item: Copy
{
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        match self.it.next() {
            None => None,
            Some(next) => {
                if let Some(lst) = self.last {
                    let pair = (lst, next);
                    self.last = Some(next);
                    Some(pair)
                } else {
                    self.last = Some(next);
                    self.next()
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}

#[allow(dead_code)]
pub fn pairs<I>(iterator: I) -> PairIter<I>
    where I: Iterator,
          I::Item: Copy
{
    PairIter {
        it: iterator,
        last: None,
    }
}

#[cfg(test)]
mod tests {

    use super::pairs;

    fn pair_vec<T: Copy>(vec: Vec<T>) -> Vec<(T,T)>{
        pairs(vec.into_iter()).collect()
    }

    #[test]
    fn test_pairs() {

        let a1 = vec![1,2,3,4];
        let a2 = vec![(1,2), (2,3), (3, 4)];

        assert_eq!(pair_vec(a1), a2);

        let b1 = vec![1,2];
        let b2 = vec![(1,2)];

        assert_eq!(pair_vec(b1), b2);

        let c1 = vec![1];
        let c2 = vec![];

        assert_eq!(pair_vec(c1), c2);

        let d1: Vec<i32> = vec![];
        let d2: Vec<(i32, i32)> = vec![];

        assert_eq!(pair_vec(d1), d2);
    }

}
