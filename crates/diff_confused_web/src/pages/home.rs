use yew::{function_component, html, Html};
use yew_router::prelude::use_navigator;

use crate::pages::Route;

#[function_component]
pub fn Home() -> Html {
    let navigator = use_navigator().unwrap();

    html! {
        <>
            <h1>{"Diff Confused Bed"}</h1>
            <div>
                <button onclick={move |_| navigator.push(&Route::Login)}>{"Login"}</button>
            </div>
        </>
    }
}
