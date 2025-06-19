pub(crate) mod protocol;

use crate::protocol::{start_node, ProtocolCommand, ProtocolHandle};
use dioxus::prelude::*;
use iroh::NodeAddr;

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

    rsx! {
        // button { onclick: move |_| handle.send(ProtocolCommand::Increment), "Increase" }
        "{signal:?}"
        Connect {}
        Library {}
    }
}

#[component]
fn Connect() -> Element {
    let mut node_id = use_signal(String::new);
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
                value: "{node_id}",
                oninput: move |evt| node_id.set(evt.value().clone()),
                placeholder: "Node ID",
            }
            button {
                onclick: move |_| {
                    println!("Connecting to {}", node_id());

                    if node_id.read().is_empty() {
                        return;
                    }

                    let Ok(node_id) = node_id.read().parse() else {
                        println!("failed to parse node id");
                        return;
                    };
                    let addr = NodeAddr::new(node_id);
                    let Ok(destination) = destination.read().parse() else {
                        println!("failed to parse destination");
                        return;
                    };

                    handle.read().send(ProtocolCommand::Download(addr, destination));
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
