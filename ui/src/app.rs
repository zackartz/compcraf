use std::collections::{HashMap, HashSet, VecDeque};

use leptos::{leptos_dom::logging::console_log, *};
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{use_websocket, UseWebsocketReturn};
use models::{Position, TurtleCommand};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_start.css"/>
        <Stylesheet id="inter" href="https://rsms.me/inter/inter.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
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

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let UseWebsocketReturn {
        ready_state,
        message,
        message_bytes,
        send,
        send_bytes,
        ..
    } = use_websocket("ws://localhost:1337/turtle_updates");

    let status = move || ready_state.get().to_string();

    // let (rows, set_rows) = create_signal(VecDeque::<Vec<String>>::with_capacity(50));
    let (rows, set_rows) = create_signal(Vec::<models::Turtle>::new());

    let send_message = store_value(move |_| {
        let action = models::QueuedAction::MovePoint(Position {
            x: -3700,
            y: 90,
            z: 1035,
        });
        let packet = serde_json::to_string(&TurtleCommand {
            turtle_id: 1,
            action,
        })
        .unwrap();
        send(&packet);
    });

    let r = create_memo(move |_| {
        let m = message.get();
        console_log(format!("m: {:?}", m).as_str());

        if m.is_none() {
            return rows.get();
        }

        let turtles: Vec<models::Turtle> = serde_json::from_str(&m.clone().unwrap()).unwrap();

        set_rows(turtles);

        rows.get()
    });

    view! {
        <div class="min-h-screen bg-black p-20">
            <h1 class="text-4xl font-bold">"Turtle Swarm"</h1>
            <p>
                "Connection: "
                <span class="p-1 bg-neutral-900 rounded-md uppercase font-mono text-green-200 px-2">
                    {status}
                </span>
            </p>
            {move || {
                r
                    .get()
                    .into_iter()
                    .enumerate()
                    .map(|(idx, t)| {
                        view! {
                            <div class="text-white p-8 mt-4 rounded-2xl border">
                                <div class="flex items-center space-x-2">
                                    <h1 class="text-2xl font-bold">"Turtle #" {t.id}<span class="ml-2 font-mono bg-neutral-900 p-2 rounded-lg">{format!("{:?}", t.curr_goal)}</span></h1>
                                </div>
                                <div class="flex">
                                    <div class="mt-2 w-[250px] space-y-1 text-xs">
                                        <div>"Direction: "<span class="py-1 px-2 bg-neutral-900 font-mono rounded-md">{format!("{:?}", t.direction)}</span></div>
                                        <div>"X: "<span class="py-1 px-2 bg-neutral-900 font-mono rounded-md">{t.pos.x}</span></div>
                                        <div>"Y: "<span class="py-1 px-2 bg-neutral-900 font-mono rounded-md">{t.pos.y}</span></div>
                                        <div>"Z: "<span class="py-1 px-2 bg-neutral-900 font-mono rounded-md">{t.pos.z}</span></div>
                                        <div>"Fuel: "<span class="py-1 px-2 bg-neutral-900 font-mono rounded-md">{t.fuel}</span></div>
                                    </div>
                                    <div class="flex justify-end w-full">
                                        // <Select />
                                        <button on:click=send_message.get_value() class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80 h-9 px-4 py-2">Move Turtle</button>
                                    </div>
                                </div>
                                <div class="max-h-54 overflow-scroll">
                                    <Table idx=idx rows=t.executed_actions/>
                                </div>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
            }}
        </div>
    }
}

#[component]
fn Select() -> impl IntoView {
    view! {
        <button class="flex h-9 items-center justify-between rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 w-[180px]"><span style="pointer-events: none;">Select Action Type</span>
        <svg width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 opacity-50" aria-hidden="true"><path d="M4.93179 5.43179C4.75605 5.60753 4.75605 5.89245 4.93179 6.06819C5.10753 6.24392 5.39245 6.24392 5.56819 6.06819L7.49999 4.13638L9.43179 6.06819C9.60753 6.24392 9.89245 6.24392 10.0682 6.06819C10.2439 5.89245 10.2439 5.60753 10.0682 5.43179L7.81819 3.18179C7.73379 3.0974 7.61933 3.04999 7.49999 3.04999C7.38064 3.04999 7.26618 3.0974 7.18179 3.18179L4.93179 5.43179ZM10.0682 9.56819C10.2439 9.39245 10.2439 9.10753 10.0682 8.93179C9.89245 8.75606 9.60753 8.75606 9.43179 8.93179L7.49999 10.8636L5.56819 8.93179C5.39245 8.75606 5.10753 8.75606 4.93179 8.93179C4.75605 9.10753 4.75605 9.39245 4.93179 9.56819L7.18179 11.8182C7.35753 11.9939 7.64245 11.9939 7.81819 11.8182L10.0682 9.56819Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path></svg>
        </button>
    }
}

#[component]
fn Table(idx: usize, rows: VecDeque<models::QueuedAction>) -> impl IntoView {
    view! {
        <table class="w-full caption-bottom text-sm">
            <caption class="mt-4 text-sm text-muted-foreground">"Recent Actions"</caption>
            <thead class="[&_tr]:border-b">
                <tr class="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px] w-[100px]">
                        "Id"
                    </th>
                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px] w-[100px]">
                        "Update"
                    </th>
                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px] w-[100px]">
                        "Goal"
                    </th>
                    <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px] w-[100px]">
                        "Fuel"
                    </th>
                </tr>
            </thead>
            <tbody class="[&_tr:last-child]:border-0">

                {rows
                    .into_iter()
                    .enumerate()
                    .rev()
                    .map(move |(idx, r)| {
                        view! {
                            <tr class="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                                <td class="p-2 align-middle [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]">
                                    {idx}
                                </td>
                                <td class="p-2 align-middle [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]">
                                    {format!("{:?}", r)}
                                </td>
                            </tr>
                        }
                    })
                    .collect::<Vec<_>>()}

            </tbody>
        </table>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! { <h1>"Not Found"</h1> }
}
