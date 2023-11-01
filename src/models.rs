use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use axum::extract::ws::Message;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio::sync::Mutex;

use models::*;

#[derive(Debug)]
pub struct TurtleController {
    pub turtle: Arc<Mutex<models::Turtle>>,
    pub client_sender: UnboundedSender<Message>,
    pub response_receiver: Receiver<String>,
}

trait PopBackAdd<T> {
    fn pop_back_add(&mut self, item: T) {}
}

impl<T> PopBackAdd<T> for VecDeque<T> {
    fn pop_back_add(&mut self, item: T) {
        if self.len() == self.capacity() {
            self.pop_front();
            self.push_back(item);
            return;
        }
        self.push_back(item)
    }
}

impl TurtleController {
    pub fn new(
        turtle: Arc<Mutex<Turtle>>,
        client_sender: UnboundedSender<Message>,
        receiver: Receiver<String>,
    ) -> Self {
        Self {
            turtle,
            client_sender,
            response_receiver: receiver,
        }
    }

    pub async fn move_blocks(
        &mut self,
        dir: MoveDirection,
        amount: i64,
    ) -> Result<(), TurtleError> {
        for _ in 0..amount {
            self.move_and_mine_block(dir).await?;
        }
        Ok(())
    }

    pub async fn turn_towards(&mut self, target_direction: Direction) -> Result<(), TurtleError> {
        let turtle = self.turtle.lock().await;
        let td = turtle.direction.clone();
        drop(turtle);

        loop {
            let turtle = self.turtle.lock().await;
            if turtle.direction == target_direction {
                break;
            }
            drop(turtle);

            match (td.clone(), target_direction.clone()) {
                (Direction::North, Direction::East)
                | (Direction::East, Direction::South)
                | (Direction::South, Direction::West)
                | (Direction::West, Direction::North) => {
                    self.turn(TurnDirection::Right).await?;
                }
                _ => {
                    self.turn(TurnDirection::Left).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn reset_north(&mut self) -> Result<(), TurtleError> {
        for _i in 0..4 {
            self.send_command(Action {
                action: ActionType::Mine(MineDirection::Forward),
            })
            .await?;
            self.send_command(Action {
                action: ActionType::Turn(TurnDirection::Right),
            })
            .await?;
        }

        let turtle = self.turtle.lock().await;
        let curr_cords = turtle.pos.clone();
        drop(turtle);

        for _i in 0..4 {
            self.move_and_mine_block(MoveDirection::Forward).await?;
            let turtle = self.turtle.lock().await;
            tracing::info!("pos: {:?}", turtle.pos);
            tracing::info!("curr_cords: {:?}", curr_cords.z);
            if curr_cords.z > turtle.pos.z {
                tracing::info!("here");
                return Ok(());
            }
            drop(turtle);
            self.move_turtle(MoveDirection::Backward).await?;
            self.send_command(Action {
                action: ActionType::Turn(TurnDirection::Right),
            })
            .await?;
        }

        Err(TurtleError::ErrorBlock)
    }

    pub async fn turn(&mut self, dir: TurnDirection) -> Result<(), TurtleError> {
        let mut turtle = self.turtle.lock().await;

        turtle.direction = match (turtle.direction.clone(), dir) {
            (Direction::North, TurnDirection::Right) => Direction::East,
            (Direction::East, TurnDirection::Right) => Direction::South,
            (Direction::South, TurnDirection::Right) => Direction::West,
            (Direction::West, TurnDirection::Right) => Direction::North,

            (Direction::North, TurnDirection::Left) => Direction::West,
            (Direction::West, TurnDirection::Left) => Direction::South,
            (Direction::South, TurnDirection::Left) => Direction::East,
            (Direction::East, TurnDirection::Left) => Direction::North,
        };

        drop(turtle);

        let _ = self
            .send_command(Action {
                action: ActionType::Turn(dir),
            })
            .await?;

        Ok(())
    }

    async fn parse_resp(&self, resp: InfoResp) {
        let mut turtle = self.turtle.lock().await;
        turtle.fuel = resp.fuel;
        turtle.blocks = resp.blocks;
        turtle.slots = resp.slots;
        turtle.pos = resp.pos;
    }

    pub async fn send_command(&mut self, command: Action) -> Result<InfoResp, TurtleError> {
        let packet = serde_json::to_string(&command).unwrap();
        let turtle = self.turtle.lock().await;
        drop(turtle);

        if self.client_sender.send(Message::Text(packet)).is_err() {
            println!("Error sending message to client. Maybe the WebSocket was closed?");
            return Err(TurtleError::ErrorWebsockets);
        }

        let resp = self.response_receiver.recv().await.unwrap();

        let resp: InfoResp = serde_json::from_str(&resp).unwrap();

        self.parse_resp(resp.clone()).await;

        return Ok(resp);
    }

    pub async fn mine_rect(&mut self, a: Position, b: Position) -> Result<(), TurtleError> {
        let turtle = self.turtle.lock().await;
        let turtle = turtle.clone();
        let mut actions = VecDeque::new();
        let mut initial_dir = turtle.direction.clone();

        for y in a.y..=b.y {
            let layer_start = Position { x: a.x, y, z: a.z };
            let layer_end = Position { x: b.x, y, z: b.z };

            actions.push_back(QueuedAction::MovePoint(layer_start));
            let mut layer_path = self.mine_layer(layer_start, layer_end).await?;
            actions.append(&mut layer_path);

            // Change initial direction for the next layer to make it a continuous snake
            initial_dir = if initial_dir == Direction::East {
                Direction::West
            } else {
                Direction::East
            };
        }

        let mut t = self.turtle.lock().await;
        t.action_queue = actions.clone();
        tracing::info!("actions {:?}", actions);

        Ok(())
    }

    async fn mine_layer(
        &self,
        start: Position,
        end: Position,
    ) -> Result<VecDeque<QueuedAction>, TurtleError> {
        let turtle = self.turtle.lock().await;
        let turtle = turtle.clone();

        let mut actions = VecDeque::new();
        let mut pos = start;
        let mut dir = turtle.direction.clone();

        while pos.y == start.y {
            if dir == Direction::East && pos.z == end.z {
                dir = Direction::South
            } else if dir == Direction::South && pos.x == end.x {
                dir = Direction::West
            } else if dir == Direction::West && pos.z == start.z {
                dir = Direction::South
            } else if dir == Direction::South && pos.x == start.x {
                dir = Direction::East
            };

            actions.push_back(QueuedAction::MoveDirection(dir));
            move_in_direction(&mut pos, &dir);

            if pos.x > end.x {
                break;
            }
        }

        Ok(actions)
    }

    pub async fn move_and_mine_block(&mut self, dir: MoveDirection) -> Result<(), TurtleError> {
        tracing::info!("attempting to move");
        let turtle = self.turtle.lock().await;
        let tblocks = turtle.blocks.clone();
        if dir == MoveDirection::Backward {
            return Err(TurtleError::ErrorBlock);
        }

        let init_pos = turtle.pos.clone();
        drop(turtle);
        loop {
            let turtle = self.turtle.lock().await;
            if turtle.pos != init_pos {
                break;
            }
            drop(turtle);

            if let Some(b) = tblocks.iter().filter(|b| b.direction == dir.swap()).next() {
                if b.exists {
                    self.send_command(Action {
                        action: ActionType::Mine(dir.swap()),
                    })
                    .await?;
                }

                self.move_turtle(dir).await?;
            }
        }

        tracing::info!("moved");

        Ok(())
    }

    pub async fn move_turtle(&mut self, dir: MoveDirection) -> Result<(), TurtleError> {
        self.send_command(Action {
            action: ActionType::Move(dir),
        })
        .await?;

        Ok(())
    }

    pub async fn move_point(&mut self, target: Position) -> Result<(), TurtleError> {
        tracing::info!("i am processing");
        let turtle = self.turtle.lock().await;
        let mut dx = target.x - turtle.pos.x;
        let mut dy = target.y - turtle.pos.y;
        let mut dz = target.z - turtle.pos.z;

        drop(turtle);

        // Check if the turtle is only one block away from its target
        if dx.abs() + dy.abs() + dz.abs() == 1 {
            if dx == 1 {
                self.turn_towards(Direction::East).await?;
            } else if dx == -1 {
                self.turn_towards(Direction::West).await?;
            } else if dz == 1 {
                self.turn_towards(Direction::South).await?;
            } else if dz == -1 {
                self.turn_towards(Direction::North).await?;
            }

            if dx.abs() == 1 || dz.abs() == 1 {
                self.move_and_mine_block(MoveDirection::Forward).await?;
            } else if dy == 1 {
                self.move_and_mine_block(MoveDirection::Up).await?;
            } else if dy == -1 {
                self.move_and_mine_block(MoveDirection::Down).await?;
            }

            return Ok(());
        }

        while dx != 0 || dy != 0 || dz != 0 {
            // Find out which axis has the longest distance remaining
            let tmp = [(dx.abs(), 'x'), (dy.abs(), 'y'), (dz.abs(), 'z')];

            let (max_val, axis) = tmp.iter().max().unwrap();

            if *max_val == 0 {
                break;
            }

            tracing::info!("thinking...");

            match axis {
                'x' => {
                    self.turn_towards(if dx > 0 {
                        Direction::East
                    } else {
                        Direction::West
                    })
                    .await?;
                    for _ in 0..dx.abs() {
                        self.move_and_mine_block(MoveDirection::Forward).await?;
                    }
                    dx = 0;
                }
                'y' => {
                    for _ in 0..dy.abs() {
                        if dy > 0 {
                            self.move_and_mine_block(MoveDirection::Up).await?;
                        } else {
                            self.move_and_mine_block(MoveDirection::Down).await?;
                        }
                    }
                    dy = 0;
                }
                'z' => {
                    self.turn_towards(if dz > 0 {
                        Direction::South
                    } else {
                        Direction::North
                    })
                    .await?;
                    for _ in 0..dz.abs() {
                        self.move_and_mine_block(MoveDirection::Forward).await?;
                    }
                    dz = 0;
                }
                _ => {}
            }
        }

        tracing::info!("completed");

        Ok(())
    }
}

fn move_in_direction(pos: &mut Position, dir: &Direction) {
    match dir {
        Direction::North => pos.x -= 1,
        Direction::South => pos.x += 1,
        Direction::East => pos.z += 1,
        Direction::West => pos.z -= 1,
    }
}

pub struct TurtleManager {
    pub turtles: Arc<Mutex<HashMap<usize, Arc<Mutex<Turtle>>>>>,
}

impl TurtleManager {
    pub fn new() -> Self {
        Self {
            turtles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_turtle(&self, turtle: Arc<Mutex<Turtle>>) {
        let mut turtles = self.turtles.lock().await;
        let id = turtle.lock().await.id;
        turtles.insert(id, turtle);
    }

    pub async fn get_turtle(&self, id: usize) -> Option<Arc<Mutex<Turtle>>> {
        let turtles = self.turtles.lock().await;
        turtles.get(&id).cloned()
    }
}
