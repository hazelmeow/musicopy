pub(crate) mod protocol;

use crate::protocol::{start_node, ProtocolCommand, ProtocolHandle};
use dioxus::prelude::*;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// A macro to make mass redeclarations of a collection of identifiers using a
/// single method more concise.
///
/// # Example
///
/// ```rs
/// // This:
/// call!(foo; bar, qux);
///
/// // Gets turned into this:
/// let bar = bar.foo();
/// let qux = qux.foo();
/// ```
#[macro_export]
macro_rules! call {
   ($method:ident; $($identifier:ident),*) => {
      $(let $identifier = $identifier.$method();)*
   }
}

/// [`call!`] but with the method set to `clone`.
#[macro_export]
macro_rules! clone {
   ($($t:tt),*) => {
      $crate::call!(clone; $($t),*);
   }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS } document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

/// Home page
#[component]
fn Home() -> Element {
    let signal = use_signal_sync(|| None);

    // start node and provide ProtocolHandle context
    let handle = use_hook(|| start_node(signal));
    use_context_provider(|| ReadOnlySignal::new(Signal::new(handle)));

    let ticket = signal
        .read()
        .as_ref()
        .and_then(|s| s.ticket.as_ref())
        .map(|t| t.to_string());

    rsx! {
        // button { onclick: move |_| handle.send(ProtocolCommand::Increment), "Increase" }
        "{signal:?}"
        Connect {}
        Library {}

        if let Some(ticket) = ticket {
            h1 { "Ticket: {ticket}" }
        }
    }
}

#[component]
fn Connect() -> Element {
    let mut ticket = use_signal(String::new);
    let mut destination = use_signal(String::new);

    let handle = use_context::<ReadOnlySignal<ProtocolHandle>>();

    rsx! {
        div {
            div {
                label { "Download Destination:" }
                input {
                    r#type: "file",
                    directory: true,
                    onchange: move |evt| {
                        if let Some(file_engine) = evt.files() {
                            let files = file_engine.files();
                            let Some(file) = files.first() else {
                                return;
                            };
                            destination.set(file.clone());
                        }
                    },
                    placeholder: "Select download folder...",
                }
            }
            input {
                r#type: "text",
                value: "{ticket}",
                oninput: move |evt| ticket.set(evt.value().clone()),
                placeholder: "Ticket",
            }
            button {
                onclick: move |_| {
                    println!("Connecting to {}", ticket());

                    if ticket.read().is_empty() {
                        return;
                    }

                    let Ok(ticket) = ticket.read().parse() else {
                        println!("failed to parse ticket");
                        return;
                    };
                    let Ok(destination) = destination.read().parse() else {
                        println!("failed to parse destination");
                        return;
                    };

                    handle.read().send(ProtocolCommand::Download(ticket, destination));
                },
                "Connect"
            }
        }
    }
}

#[component]
fn Library() -> Element {
    let mut paths = use_signal(Vec::<String>::new);

    let handle = use_context::<ReadOnlySignal<ProtocolHandle>>();

    rsx! {
        div {
            h2 { "Library Paths" }
            ul {
                for (idx, path) in paths().iter().enumerate() {
                    li {
                        "{path} "
                        button {
                            onclick: move |_| {
                                paths.remove(idx);
                                handle.read().send(ProtocolCommand::Scan(paths.read().clone()));
                            },
                            "Remove"
                        }
                    }
                }
            }
            input {
                r#type: "file",
                directory: true,
                onchange: move |evt| {
                    if let Some(file_engine) = evt.files() {
                        let files = file_engine.files();
                        for file_name in files {
                            paths.push(file_name);
                        }
                        handle.read().send(ProtocolCommand::Scan(paths.read().clone()));
                    }
                }
            }
        }
    }
}

/// Blog page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div {
            "page 2"
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div {
            id: "navbar",
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Blog { id: 1 },
                "Blog"
            }
        }

        Outlet::<Route> {}
    }
}
