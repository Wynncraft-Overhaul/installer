// TODO: Make into a component
use std::fmt::Debug;

use dioxus::prelude::*;
use log::warn;

#[derive(Default)]
struct ModalInfo {
    title: String,
    contents: Element,
    open: bool,
    cancelable: bool,
    callback: Option<CopyValue<Box<dyn FnMut(bool)>>>,
}

#[derive(Clone, Default)]
pub struct ModalContext {
    inner: Signal<ModalInfo>,
    canceled: bool,
}

impl ModalContext {
    pub fn open<F: FnMut(bool) + 'static, T: Into<String>>(
        &mut self,
        title: T,
        contents: Element,
        cancelable: bool,
        callback: Option<F>,
    ) {
        let title = title.into();
        if self.inner.read().open {
            warn!(
                "Tried to open modal: '{}' while modal: '{}' is open!",
                title,
                self.inner.read().title
            );
            return;
        }
        self.inner.with_mut(|inner| {
            inner.title = title;
            inner.contents = contents;
            inner.cancelable = cancelable;
            if let Some(callback) = callback {
                inner.callback = Some(CopyValue::new(Box::new(callback)));
            } else {
                inner.callback = None;
            }
            inner.open = true;
        });
        self.canceled = false;
    }

    pub fn cancel(&mut self) {
        self.canceled = true;
        self.close();
    }

    pub fn close(&mut self) {
        if let Some(callback) = self.inner.read().callback {
            callback.write_unchecked()(self.canceled);
        }
        self.inner.write().open = false;
    }
}

impl Debug for ModalContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModalContext {{
            title: \"{}\",
            contents: {:#?},
            open: {},
            cancelable: {},
            callback: {},
            canceled: {},
        }}",
            self.inner.read().title,
            self.inner.read().contents,
            self.inner.read().open,
            self.inner.read().cancelable,
            if self.inner.read().callback.is_some() {
                "Some"
            } else {
                "None"
            },
            self.canceled
        )
    }
}

#[component]
pub fn Modal() -> Element {
    let modal = use_context::<ModalContext>();
    rsx!(
        div {
            class: "modal-backdrop",
            hidden: !modal.inner.read().open,
            onclick: {
                let mut modal = modal.clone();
                move |_| modal.cancel()
            }
        }
        dialog { class: "modal", open: modal.inner.read().open,
            h1 { class: "modal-title", "{modal.inner.read().title}" }
            div { class: "modal-contents", {modal.inner.read().contents.clone()} }
            div { class: "modal-button-layout",
                button {
                    class: "modal-button",
                    onclick: {
                        let mut modal = modal.clone();
                        move |_| modal.close()
                    },
                    autofocus: true,
                    if modal.inner.read().cancelable {
                        "Continue"
                    } else {
                        "Close"
                    }
                }
                if modal.inner.read().cancelable {
                    button {
                        class: "modal-button cancel",
                        onclick: {
                            let mut modal = modal.clone();
                            move |_| modal.cancel()
                        },
                        autofocus: true,
                        "Cancel"
                    }
                }
            }
        }
    )
}
