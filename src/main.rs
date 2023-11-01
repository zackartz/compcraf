use ::models::{
    Action, ActionType, Direction, Goal, Material, MoveDirection, Position, QueuedAction, Turtle,
    TurtleCommand, TurtleError,
};
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use models::TurtleController;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::models::TurtleManager;

mod models;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let manager = Arc::new(models::TurtleManager::new());
    let app = Router::new()
        .route("/ws", get(handle_connection))
        .route("/turtle_updates", get(handle_turtle_updates))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(manager);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1337").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn handle_turtle_updates(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<TurtleManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut ws_tx, mut ws_rx) = socket.split();

        tracing::info!("recieved connection");

        let m = manager.clone();

        tokio::spawn(async move {
            loop {
                if let Some(msg) = ws_rx.next().await {
                    tracing::info!("recieved msg");
                    if msg.as_ref().unwrap().to_text().unwrap() == "" {
                        continue;
                    }

                    let cmd: TurtleCommand =
                        serde_json::from_str(&msg.unwrap().to_text().unwrap()).unwrap();
                    let mut t = m.turtles.lock().await;
                    tracing::info!("{:?}", t);
                    t.get_mut(&cmd.turtle_id)
                        .unwrap()
                        .lock()
                        .await
                        .action_queue
                        .push_front(cmd.action);
                    tracing::info!("pushed action");
                }
            }
        });

        tracing::info!("hi");

        tokio::spawn(async move {
            loop {
                let all_turtle_data = {
                    let turtles = manager.turtles.lock().await;
                    let mut ts = Vec::new();
                    for value in turtles.values() {
                        ts.push(value.lock().await.clone());
                    }
                    ts
                };

                let update_message =
                    Message::Text(serde_json::to_string(&all_turtle_data).unwrap());
                ws_tx.send(update_message).await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    })
}

async fn event_loop(
    turtle_controller: Arc<tokio::sync::Mutex<TurtleController>>,
) -> Result<(), TurtleError> {
    turtle_controller.lock().await.reset_north().await.unwrap();
    loop {
        tracing::info!("hello world");
        let turtle = {
            let tc = turtle_controller.lock().await;
            let turtle = tc.turtle.lock().await.clone();
            turtle
        };

        // if turtle.fuel < 50 {
        //     tracing::info!("STOPPED MINING. NEEDS FUEL");
        //     let tc = turtle_controller.lock().await;
        //     tc.turtle.lock().await.curr_goal = Goal::Refuel;
        // } else if turtle.main_goal != turtle.curr_goal && turtle.curr_goal != Goal::Deposit {
        //     let tc = turtle_controller.lock().await;
        //     tc.turtle.lock().await.curr_goal = turtle.main_goal;
        // }
        //
        // if turtle.needs_deposit() {
        //     let tc = turtle_controller.lock().await;
        //     tc.turtle.lock().await.curr_goal = turtle.main_goal;
        //     tc.turtle.lock().await.prev_dir = turtle.direction.clone();
        //     tc.turtle.lock().await.prev_pos = turtle.pos;
        //     tc.turtle.lock().await.curr_goal = Goal::Deposit;
        // }

        match turtle.curr_goal {
            Goal::Idle => {
                tracing::info!("sdfasfda");
                if turtle.action_queue.len() > 0 {
                    tracing::info!("fuck");
                    let mut tc = turtle_controller.lock().await;
                    let action = tc.turtle.lock().await.action_queue.pop_front().unwrap();

                    tracing::info!("PROCESSING QA: {:?}", action);
                    match action {
                        QueuedAction::Turn(dir) => tc.turn(dir).await?,
                        QueuedAction::MoveAndMineLen(l) => {
                            tc.move_blocks(MoveDirection::Forward, l).await?
                        }
                        QueuedAction::MoveDirection(d) => {
                            tc.turn_towards(d).await?;
                            tc.move_and_mine_block(MoveDirection::Forward).await?;
                        }
                        QueuedAction::MovePoint(p) => tc.move_point(p).await?,
                        QueuedAction::TurnToward(d) => tc.turn_towards(d).await?,
                        QueuedAction::MoveAndMine(d) => tc.move_and_mine_block(d).await?,
                        QueuedAction::Nothing => {}
                    };
                    drop(tc);

                    let tc = turtle_controller.lock().await;
                    tc.turtle.lock().await.executed_actions.push_back(action);
                }

                let mut tc = turtle_controller.lock().await;
                tc.send_command(Action {
                    action: ActionType::Info,
                })
                .await?;
            }
            Goal::Refuel => {
                let mut tc = turtle_controller.lock().await;
                tc.send_command(Action {
                    action: ActionType::Info,
                })
                .await?;

                let slot = turtle.slots.iter().find(|s| {
                    let t = s.type_field.clone().unwrap();

                    if t.name == "minecraft:coal" {
                        return true;
                    }

                    if t.name == "minecraft:charcoal" {
                        return true;
                    }

                    return false;
                });

                // if turtle.fuel > 50 {
                //     turtle.curr_goal = turtle.main_goal;
                // }
                //
                // if slot.is_none() {
                //     turtle.set_level(Material::Coal as i64).await?;
                //     turtle.move_turtle(MoveDirection::Forward).await?;
                //
                //     continue;
                // }
                if slot.is_some() {
                    turtle_controller
                        .lock()
                        .await
                        .send_command(Action {
                            action: ActionType::Slot {
                                name: "Select".to_string(),
                                args: vec![slot.unwrap().id],
                            },
                        })
                        .await?;
                    turtle_controller
                        .lock()
                        .await
                        .send_command(Action {
                            action: ActionType::Refuel,
                        })
                        .await?;
                }

                // if turtle.fuel > 50 {
                //     let tc = turtle_controller.lock().await;
                //     let mut turtle = tc.turtle.lock().await;
                //     turtle.curr_goal = turtle.main_goal;
                // }
                tracing::info!("here...");
            }
            Goal::Deposit => {}
            Goal::Mine(_d) => {}
        }
    }
}

async fn handle_connection(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<TurtleManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut ws_tx, mut ws_rx) = socket.split();

        let (turtle_tx, mut turtle_rx) = unbounded_channel();
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(32); // Create a channel for responses

        let turtle = Arc::new(tokio::sync::Mutex::new(Turtle {
            id: 1,
            pos: Position { x: 0, y: 0, z: 0 },
            fuel: 0,
            slots: vec![],
            blocks: vec![],
            visited: HashSet::new(),
            prev_pos: Position { x: 0, y: 0, z: 0 },
            prev_dir: Direction::North,
            direction: Direction::North,
            curr_goal: Goal::Idle,
            main_goal: Goal::Idle,
            mine_area: vec![],
            action_list: vec![],
            action_queue: VecDeque::new(),
            executed_actions: VecDeque::with_capacity(100),
        }));

        let turtle_controller =
            TurtleController::new(turtle.clone(), turtle_tx.clone(), response_rx);

        manager.add_turtle(turtle.clone()).await;

        tokio::spawn(event_loop(Arc::new(tokio::sync::Mutex::new(
            turtle_controller,
        ))));

        let response_tx_for_ws = response_tx.clone();
        tokio::spawn(async move {
            while let Some(message) = ws_rx.next().await {
                match message {
                    Ok(msg) => {
                        if let Message::Text(text) = msg {
                            response_tx_for_ws.send(text).await.unwrap(); // Send received messages to the response channel
                        }
                    }
                    Err(_) => {
                        println!("WebSocket error. Exiting.");
                        break;
                    }
                }
            }
        });

        while let Some(update) = turtle_rx.recv().await {
            let _ = ws_tx.send(update).await;
        }
    })
}
