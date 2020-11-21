#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Millisecond(pub i128);

impl Millisecond {
    pub fn as_secs(self) -> f32 {
        self.0 as f32 / 1000.
    }

    pub fn as_minutes(self) -> f32 {
        self.0 as f32 / 1000. / 60.
    }
}

impl std::ops::Sub for Millisecond {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Millisecond(self.0 - rhs.0)
    }
}

impl std::ops::Add for Millisecond {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Millisecond(self.0 + rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimpleClock {
    turn_time: Millisecond,
}

impl SimpleClock {
    fn clock(&self) -> PlayerClock {
        PlayerClock::Plain {
            last_time: Millisecond(0),
            time_left: self.turn_time,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FischerClock {
    pub main_time: Millisecond,
    pub increment: Millisecond,
}

impl FischerClock {
    fn clock(&self) -> PlayerClock {
        PlayerClock::Plain {
            last_time: Millisecond(0),
            time_left: self.main_time,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClockRule {
    /// Simple time gives the player exactly `turn_time` milliseconds per turn.
    Simple(SimpleClock),
    /// Fischer time adds `increment` milliseconds to the player's clock after making an action.
    Fischer(FischerClock),
}

impl ClockRule {
    fn clock(&self) -> PlayerClock {
        match self {
            ClockRule::Simple(rule) => rule.clock(),
            ClockRule::Fischer(rule) => rule.clock(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PlayerClock {
    /// Only main time
    Plain {
        last_time: Millisecond,
        time_left: Millisecond,
    },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameClock {
    /// One clock per team or player. This is decided by the game controller, this module doesn't care which is used.
    pub clocks: Vec<PlayerClock>,
    pub rule: ClockRule,
    pub paused: bool,
}

impl GameClock {
    pub fn new(rule: ClockRule, clock_count: usize) -> Self {
        GameClock {
            clocks: vec![rule.clock(); clock_count],
            rule,
            paused: true,
        }
    }

    pub fn initialize_clocks(&mut self, initial_time: Millisecond) {
        for clock in &mut self.clocks {
            match clock {
                PlayerClock::Plain { last_time, .. } => {
                    *last_time = initial_time;
                }
            }
        }
    }

    /// Returns the time left for the given clock at current timestamp `time`.
    pub fn advance_clock(&mut self, clock_idx: usize, time: Millisecond) -> Millisecond {
        if self.paused {
            return Millisecond(0);
        }

        let clock = &mut self.clocks[clock_idx];

        match clock {
            PlayerClock::Plain {
                last_time,
                time_left,
            } => {
                let duration = time - *last_time;
                *time_left = *time_left - duration;
                *time_left
            }
        }
    }

    pub fn end_turn(&mut self, clock_idx: usize, time: Millisecond) {
        if self.paused {
            return;
        }

        let clock = &mut self.clocks[clock_idx];

        match &mut self.rule {
            ClockRule::Simple(rule) => match clock {
                PlayerClock::Plain { time_left, .. } => {
                    *time_left = rule.turn_time;
                }
            },
            ClockRule::Fischer(rule) => match clock {
                PlayerClock::Plain { time_left, .. } => {
                    *time_left = *time_left + rule.increment;
                }
            },
        }

        for clock in &mut self.clocks {
            match clock {
                PlayerClock::Plain { last_time, .. } => {
                    *last_time = time;
                }
            }
        }
    }

    pub fn pause(&mut self, paused: bool) {
        self.paused = paused;
    }
}
