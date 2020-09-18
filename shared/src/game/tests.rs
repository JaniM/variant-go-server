use super::*;

#[test]
fn seats() {
    let mut game = Game::standard(
        &vec![1, 2],
        GroupVec::from(&[0, 15][..]),
        (9, 9),
        GameModifier::default(),
    )
    .unwrap();

    assert_eq!(
        &game.shared.seats,
        &GroupVec::from(
            &[
                Seat {
                    player: None,
                    team: Color(1),
                    resigned: false,
                },
                Seat {
                    player: None,
                    team: Color(2),
                    resigned: false,
                },
            ][..]
        )
    );

    game.take_seat(100, 0).expect("Take seat");
    game.take_seat(200, 1).expect("Take seat");

    assert_eq!(
        &game.shared.seats,
        &GroupVec::from(
            &[
                Seat {
                    player: Some(100),
                    team: Color(1),
                    resigned: false,
                },
                Seat {
                    player: Some(200),
                    team: Color(2),
                    resigned: false,
                },
            ][..]
        )
    );

    assert_eq!(game.take_seat(300, 2), Err(TakeSeatError::DoesNotExist));
    assert_eq!(game.take_seat(300, 1), Err(TakeSeatError::NotOpen));
    assert_eq!(game.leave_seat(300, 1), Err(TakeSeatError::NotOpen));
}

use insta::{assert_debug_snapshot, glob};
use std::fs;

#[test]
fn replay_snapshots() {
    glob!("replays/*.txt", |path| {
        let input = fs::read(path).unwrap();
        let game = Game::load(&input).unwrap();
        let view = game.get_view(0);
        assert_debug_snapshot!(view);
    });
}
