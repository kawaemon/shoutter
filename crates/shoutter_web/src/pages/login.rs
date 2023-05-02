use std::ops::Deref;

use web_sys::HtmlInputElement;
use yew::html::onchange::Event;
use yew::{function_component, html, use_state, Callback, Html, TargetCast, UseStateHandle};

fn onchange_for(state: &UseStateHandle<String>) -> Callback<Event, ()> {
    let state = state.clone();
    Callback::from(move |e: Event| {
        state.set(e.target_dyn_into::<HtmlInputElement>().unwrap().value());
    })
}

#[function_component]
pub fn Login() -> Html {
    let user_id = use_state(String::new);
    let password = use_state(String::new);

    html! {
        <>
            <h1>{"Login"}</h1>
            <div>
                {"UserID"}
                <input
                    type="text"
                    onchange={onchange_for(&user_id)}
                    value={user_id.deref().clone()}
                />
            </div>
            <div>
                {"Password"}
                <input
                    type="password"
                    onchange={onchange_for(&password)}
                    value={password.deref().clone()}
                />
            </div>
        </>
    }
}
