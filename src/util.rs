use std::iter::IntoIterator;
use std::ops;

pub trait Join<T> {
    fn join(self, T) -> String;
}

impl<'a, T> Join<&'a str> for T
where
    T: IntoIterator,
    <T as IntoIterator>::Item: AsRef<str>,
{
    fn join(self, joiner: &str) -> String {
        self.into_iter()
            .enumerate()
            .fold(String::new(), |mut acc, (idx, item)| {
                if idx > 0 {
                    acc.push_str(joiner);
                }
                acc.push_str(item.as_ref());
                acc
            })
    }
}

impl<T> Join<char> for T
where
    T: IntoIterator,
    <T as IntoIterator>::Item: AsRef<str>,
{
    fn join(self, joiner: char) -> String {
        self.into_iter()
            .enumerate()
            .fold(String::new(), |mut acc, (idx, item)| {
                if idx > 0 {
                    acc.push(joiner);
                }
                acc.push_str(item.as_ref());
                acc
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range<T> {
    pub start: Option<T>,
    pub end: Option<T>,
}

impl<T> From<ops::Range<T>> for Range<T> {
    fn from(ops::Range { start, end }: ops::Range<T>) -> Range<T> {
        Range {
            start: Some(start),
            end: Some(end),
        }
    }
}

impl<T> From<ops::RangeFrom<T>> for Range<T> {
    fn from(ops::RangeFrom { start }: ops::RangeFrom<T>) -> Range<T> {
        Range {
            start: Some(start),
            end: None,
        }
    }
}

impl<T> From<ops::RangeTo<T>> for Range<T> {
    fn from(ops::RangeTo { end }: ops::RangeTo<T>) -> Range<T> {
        Range {
            start: None,
            end: Some(end),
        }
    }
}

impl<T> From<ops::RangeFull> for Range<T> {
    fn from(_: ops::RangeFull) -> Range<T> {
        Range {
            start: None,
            end: None,
        }
    }
}
