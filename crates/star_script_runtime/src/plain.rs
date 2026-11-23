//! Plain values: what `parallel` functions take and give, copied to their
//! thread and back.

use crate::List;

/// A StarScript value that can be copied to another thread: numbers,
/// `bool`, `string`, and lists and nullables of them. Lists are shared on
/// one thread (`Rc`), so they travel as vectors of copies.
pub trait Plain: Sized {
    /// The copy that travels between threads.
    type Sent: Send + 'static;

    /// A copy of the value, to send to another thread.
    fn send(&self) -> Self::Sent;

    /// The value again, from a copy received from another thread.
    fn receive(sent: Self::Sent) -> Self;
}

/// Values that are already their own copy.
macro_rules! plain_itself {
    ($($ty:ty),*) => {
        $(impl Plain for $ty {
            type Sent = Self;

            fn send(&self) -> Self {
                self.clone()
            }

            fn receive(sent: Self) -> Self {
                sent
            }
        })*
    };
}

plain_itself!(i16, i32, i64, f32, f64, bool, String);

impl<T: Plain> Plain for Option<T> {
    type Sent = Option<T::Sent>;

    fn send(&self) -> Self::Sent {
        self.as_ref().map(Plain::send)
    }

    fn receive(sent: Self::Sent) -> Self {
        sent.map(T::receive)
    }
}

impl<T: Plain + Clone> Plain for List<T> {
    type Sent = Vec<T::Sent>;

    fn send(&self) -> Self::Sent {
        self.to_vec().iter().map(Plain::send).collect()
    }

    fn receive(sent: Self::Sent) -> Self {
        Self::from_vec(sent.into_iter().map(T::receive).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_lists_travel_as_copies() {
        let inner = List::from_vec(vec![Some(1_i64), None]);
        let outer = List::from_vec(vec![inner.clone()]);

        let received = List::<List<Option<i64>>>::receive(outer.send());
        inner.push(Some(3));

        assert_eq!(received.get(0).to_vec(), [Some(1), None]);
        assert_eq!(received.length(), 1);
    }

    #[test]
    fn texts_and_numbers_are_their_own_copy() {
        assert_eq!(String::receive("hi".to_owned().send()), "hi");
        assert!(bool::receive(true.send()));
        assert!((f32::receive(1.5_f32.send()) - 1.5).abs() < f32::EPSILON);
    }
}
