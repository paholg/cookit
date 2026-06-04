use dioxus::prelude::*;

#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        nav { id: "navbar", {children} }
    }
}
