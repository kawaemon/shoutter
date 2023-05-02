use shoutter_api_interface::{protobuf, GreetingEndpoint};
use stylist::yew::use_style;
use yew::{function_component, html, Html};
use yew_router::prelude::use_navigator;

use crate::hooks::api_client::{use_safe_endpoint, RequestState};
use crate::pages::Route;

#[function_component]
pub fn Home() -> Html {
    let navigator = use_navigator().unwrap();
    let greeting = use_safe_endpoint(
        GreetingEndpoint,
        (),
        protobuf::GreetingName { name: "a".into() },
    );
    let code_style = use_style! {
        display: block;
        white-space: pre-wrap;
    };

    html! {
        <>
            <h1>{"Diff Confused Bed"}</h1>
            <div>
                <button onclick={move |_| navigator.push(&Route::Login)}>{"Login"}</button>
            </div>
            <code class={code_style}>{
                match &*greeting {
                    RequestState::Pending => "Pending".to_owned(),
                    RequestState::Ok(d) => d.content.clone(),
                    RequestState::Err(e) => format!("Error:\n{e:?}")
                }
            }</code>
        </>
    }
}
