use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::VecDeque;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Copy, Eq, Hash)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub direction: MineDirection,
    pub exists: bool,
    pub block: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum Material {
    Coal = 50,
    Diamond = -53,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum Goal {
    Mine(Material),
    Refuel,
    Deposit,
    Idle,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum MoveDirection {
    Forward,
    Backward,
    Up,
    Down,
}

impl MoveDirection {
    pub fn swap(&self) -> MineDirection {
        match self {
            MoveDirection::Forward => MineDirection::Forward,
            MoveDirection::Up => MineDirection::Up,
            MoveDirection::Down => MineDirection::Down,
            MoveDirection::Backward => panic!("Cannot swap backward"),
        }
    }

    pub fn reverse(&self) -> MoveDirection {
        match self {
            MoveDirection::Forward => MoveDirection::Backward,
            MoveDirection::Up => MoveDirection::Down,
            MoveDirection::Down => MoveDirection::Up,
            MoveDirection::Backward => MoveDirection::Forward,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub enum TurnDirection {
    Left,
    Right,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TurtleCommand {
    pub action: QueuedAction,
    pub turtle_id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueuedAction {
    MoveAndMine(MoveDirection),
    MoveDirection(Direction),
    MoveAndMineLen(i64),
    MovePoint(Position),
    Turn(TurnDirection),
    TurnToward(Direction),
    Nothing,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub id: i64,
    #[serde(rename = "type")]
    pub type_field: Option<Type>,
    pub space: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Type {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Action {
    pub action: ActionType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum MineDirection {
    Forward,
    Down,
    Up,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum ChestAction {
    Deposit,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum ActionType {
    Move(MoveDirection),
    Turn(TurnDirection),
    Mine(MineDirection),
    Refuel,
    Interact,
    Chest(ChestAction),
    Slot { name: String, args: Vec<i64> },
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Turtle {
    pub id: usize,
    pub pos: Position,
    pub direction: Direction,
    pub blocks: Vec<Block>,
    pub curr_goal: Goal,
    pub slots: Vec<Slot>,
    pub fuel: i64,
    pub main_goal: Goal,
    pub action_list: Vec<Action>,
    pub visited: HashSet<Position>,
    pub mine_area: Vec<Position>,
    pub action_queue: VecDeque<QueuedAction>,
    pub prev_pos: Position,
    pub prev_dir: Direction,
    pub executed_actions: VecDeque<QueuedAction>,
}

impl Turtle {
    fn calculate_space(&self) -> f64 {
        let max = self.slots.iter().map(|s| s.space).sum::<i64>() as f64;
        let current = self
            .slots
            .iter()
            .map(|s| {
                let mut count: i64 = 0;
                if s.type_field.is_some() {
                    count = s.type_field.as_ref().unwrap().count;
                }
                count as f64
            })
            .sum::<f64>();

        max / current
    }

    pub fn needs_deposit(&self) -> bool {
        (self.calculate_space() < 0.25
            || self
                .slots
                .iter()
                .filter(|s| s.type_field.is_some())
                .collect::<Vec<_>>()
                .len()
                >= 16)
            && self.curr_goal != Goal::Refuel
            && self.curr_goal != Goal::Deposit
    }
}

#[derive(Debug)]
pub enum TurtleError {
    ErrorNoFuel,
    ErrorBlock,
    ErrorWebsockets,
    ErrorParsing(String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chest {
    pub slots: Vec<Slot2>,
    pub size: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot2 {
    pub name: String,
    pub count: i64,
    pub nbt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResp {
    pub chest: Option<Chest>,
    pub fuel: i64,
    pub slots: Vec<Slot>,
    pub blocks: Vec<Block>,
    pub pos: Position,
}
