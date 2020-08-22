pub trait AssumeFrom<T> {
    fn assume(x: &T) -> &Self;
    fn assume_mut(x: &mut T) -> &mut Self;
}

#[macro_export]
macro_rules! assume {
    ($owner:ident, $var:pat => $out:expr, $ty:ty) => {
        impl AssumeFrom<$owner> for $ty {
            fn assume(x: &$owner) -> &$ty {
                use $owner::*;
                match x {
                    $var => $out,
                    _ => panic!(concat!("Assumed ", stringify!($var), " but was in {:?}"), x),
                }
            }

            fn assume_mut(x: &mut $owner) -> &mut $ty {
                use $owner::*;
                match x {
                    $var => $out,
                    _ => panic!(concat!("Assumed ", stringify!($var), " but was in {:?}"), x),
                }
            }
        }
    };
    ($owner:ident) => {
        impl $owner {
            fn assume<T: AssumeFrom<Self>>(&self) -> &T {
                T::assume(self)
            }

            fn assume_mut<T: AssumeFrom<Self>>(&mut self) -> &mut T {
                T::assume_mut(self)
            }
        }
    };
}
