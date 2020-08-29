#[macro_use]
mod assume;
pub mod game;
pub mod message;
pub mod states;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
