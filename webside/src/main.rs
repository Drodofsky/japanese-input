mod components;
mod data;
mod outcome;
mod pages;
mod recognize_answer;
mod vocab_data;

use std::rc::Rc;

use dioxus::prelude::*;

use data::{AppData, AppDataHandle};
use pages::{AnalyzePage, HomePage, RecognizePage, ReviewPage};

const STYLESHEET: Asset = asset!("/assets/style.css");

fn main() {
    dioxus::launch(app);
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Recognize,
    Analyze,
    Review,
}

/// Debug-only override for the light/dark theme — just the two real states; which one is "first" is decided at startup from the OS/browser's actual `prefers-color-scheme`, not hardcoded, so the toggle starts in sync with the system and only diverges once you click it.
#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn attr(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Theme::Light => "☀️ Light",
            Theme::Dark => "🌙 Dark",
        }
    }

    fn toggled(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

fn app() -> Element {
    use_context_provider::<AppDataHandle>(|| Rc::new(AppData::load()));
    let mut page = use_signal(|| Page::Home);
    let mut theme = use_signal(|| Theme::Light);

    // Runs once on mount (no signal reads outside the spawned task, so this effect has no reactive dependencies to re-fire on): reads the actual system preference and seeds `theme` from it, instead of hardcoding a default. Until this resolves the button briefly assumes Light.
    use_effect(move || {
        spawn(async move {
            let is_dark = document::eval(
                "return window.matchMedia('(prefers-color-scheme: dark)').matches;",
            )
            .join::<bool>()
            .await
            .unwrap_or(false);
            theme.set(if is_dark { Theme::Dark } else { Theme::Light });
        });
    });

    // Reactive: applies whichever theme is current, whether that came from the system-detection above or a later click on the toggle button.
    use_effect(move || {
        let attr = theme().attr();
        document::eval(&format!(
            "document.documentElement.setAttribute('data-theme', '{attr}')"
        ));
    });

    let tab_class = |target: Page| {
        if page() == target {
            "tab active"
        } else {
            "tab"
        }
    };

    rsx! {
        document::Stylesheet { href: STYLESHEET }
        nav { class: "tab-nav",
            button { class: tab_class(Page::Home), onclick: move |_| page.set(Page::Home), "Home" }
            button {
                class: tab_class(Page::Recognize),
                onclick: move |_| page.set(Page::Recognize),
                "Recognition"
            }
            button {
                class: tab_class(Page::Analyze),
                onclick: move |_| page.set(Page::Analyze),
                "Kanji Analysis"
            }
            button {
                class: tab_class(Page::Review),
                onclick: move |_| page.set(Page::Review),
                "Vocab Review"
            }
            button {
                class: "tab theme-toggle",
                onclick: move |_| theme.set(theme().toggled()),
                title: "Debug: toggle light/dark theme",
                "{theme().label()}"
            }
        }
        main { class: "page",
            match page() {
                Page::Home => rsx! { HomePage {} },
                Page::Analyze => rsx! { AnalyzePage {} },
                Page::Recognize => rsx! { RecognizePage {} },
                Page::Review => rsx! { ReviewPage {} },
            }
        }
        footer {
            a { href: "https://github.com/Drodofsky/japanese-input", target: "_blank", "japanese-input" }
            " — source on GitHub"
        }
    }
}
