use dioxus::prelude::*;

/// Rendered from the addon's own `readme.md` (this repo's root) by `build.rs` at build time, split around its one embedded image so it can be spliced with `BACK_SCREENSHOT` below — see `build.rs::render_readme` for why this isn't done at runtime.
const README_BEFORE: &str = include_str!(concat!(env!("OUT_DIR"), "/readme_before.html"));
const README_AFTER: &str = include_str!(concat!(env!("OUT_DIR"), "/readme_after.html"));

/// `build.rs` mirrors this repo's own `media/` folder into `assets/media` on every build (never committed — always freshly copied), so this can reference the same screenshot the README does.
const BACK_SCREENSHOT: Asset = asset!("/assets/media/back.png");

#[component]
pub fn HomePage() -> Element {
    rsx! {
        div { class: "card readme",
            div { dangerous_inner_html: "{README_BEFORE}" }
            img {
                class: "readme-screenshot",
                src: BACK_SCREENSHOT,
                alt: "Screenshot of the japanese-input addon drawing UI in the Anki reviewer",
            }
            div { dangerous_inner_html: "{README_AFTER}" }
        }
    }
}
