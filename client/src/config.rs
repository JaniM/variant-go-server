macro_rules! config {
    ($name:ident, $default:expr) => {
        pub(crate) const $name: &str = match option_env!(stringify!($name)) {
            Some(s) => s,
            None => $default,
        };
    };
}

config!(WS_URL, "ws://localhost:8088/ws/");

// Give `konst` crate a try
pub(crate) const CONN_RETRY_DELAY: u32 = 1000;