macro_rules! config {
    ($name:ident, $default:expr) => {
        pub(crate) const $name: &str = match option_env!(stringify!($name)) {
            Some(s) => s,
            None => $default,
        };
    };
}

config!(WS_URL, "ws://localhost:8088/ws/");

// Give `konst` crate a try for parsing these
pub(crate) const CONN_RETRY_DELAY: u32 = 1000;
pub(crate) const SIDEBAR_SIZE: i32 = 300;
