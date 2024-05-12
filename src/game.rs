#[derive(Debug, PartialEq)]
pub enum GameResult {
    Correct,
    Incorrect,
}

impl From<bool> for GameResult {
    fn from(value: bool) -> Self {
        match value {
            true => GameResult::Correct,
            false => GameResult::Incorrect,
        }
    }
}

pub trait Game {
    fn new_game(&mut self);
    fn advance(&mut self) -> u8;
    fn play(&mut self) -> GameResult;
}
