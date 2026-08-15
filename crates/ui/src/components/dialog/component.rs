use {
    dioxus::prelude::*,
    dioxus_primitives::dialog::{self, DialogDescriptionProps, DialogRootProps, DialogTitleProps},
};

#[css_module("/src/components/dialog/style.css")]
struct Styles;

#[component]
pub fn Dialog(props: DialogRootProps) -> Element {
    rsx! {
        dialog::DialogRoot {
            class: Styles::dx_dialog_backdrop,
            id: props.id,
            is_modal: props.is_modal,
            default_open: props.default_open,
            open: props.open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            dialog::DialogContent {
                class: Styles::dx_dialog.to_string(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    rsx! {
        dialog::DialogTitle {
            class: Styles::dx_dialog_title,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    rsx! {
        dialog::DialogDescription {
            class: Styles::dx_dialog_description,
            attributes: props.attributes,
            {props.children}
        }
    }
}
