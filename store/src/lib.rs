mod store;

pub use crate::store::*;

/// Generates boilerplate for using a store.
///
/// Usage:
///
/// ```ignore
/// store! {
///     store StoreName,
///     state StoreState,
///     request RequestEnum {
///         method => Variant(a: T, b: U)
///     }
/// }
/// ```
#[macro_export]
macro_rules! store {
    (
        store $store:ident,
        state $state:ident,
        request $name:ident { $(
            $fn:ident => $var:ident $( ( $(
                $arg:ident : $argty:ty
            ),+ ) )?
        ),* $(,)? }
    ) => {
        pub enum $name {
            $(
                $var $( ( $($argty),+ ) )?
            ),*
        }

        pub struct $store {
            bridge: StoreBridge<$state>,
        }

        impl $store {
            pub fn bridge(cb: Callback<<StoreWrapper<$state> as Agent>::Output>) -> Self {
                Self {
                    bridge: <$state as Bridgeable>::bridge(cb),
                }
            }
        }

        impl $store {
            $(
                pub fn $fn ( &mut self, $( $($arg : $argty ),+ )? ) {
                    self.bridge.send($name::$var $( (
                        $($arg),+
                    ) )? );
                }
            )*
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
