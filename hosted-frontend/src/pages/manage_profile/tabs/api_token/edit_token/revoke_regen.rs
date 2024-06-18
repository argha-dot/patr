use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::{
	pages::{TokenModal, TokenParams},
	prelude::*,
};

#[component]
pub fn RevokeApiToken() -> impl IntoView {
	let revoke_api_token_action = create_server_action::<RevokeApiTokenFn>();
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let params = use_params::<TokenParams>();
	let token_id = create_rw_signal(params.with(|params| {
		params
			.as_ref()
			.map(|param| param.token_id.clone().unwrap_or_default())
			.unwrap_or_default()
	}));

	let show_revoke_modal = create_rw_signal(false);

	view! {
		<Show
			when=move || show_revoke_modal.get()
		>
			<Modal
				color_variant={SecondaryColorVariant::Light}
			>
				<div class="center-modal txt-white txt-sm fc-fs-fs bg-secondary-light br-sm p-xl show-center-modal">
					<h3 class="txt-primary txt-lg">"Revoke API Token?"</h3>
					<p class="txt-sm txt-thin my-md">
						"This Process "
						<strong>"CANNOT "</strong>
						"be undone!"
					</p>
					// <div class="full-width fr-fs-fs f-wrap px-md py-xs bg-secondary-medium br-sm of-hidden">
					// </div>
					<div class="mt-lg full-width fr-fe-ct gap-md">
						<button class="btn-plain txt-white" on:click={move |_| show_revoke_modal.set(false)}>"CANCEL"</button>
						<ActionForm action={revoke_api_token_action}>
							<input type="hidden" name="access_token" prop:value={access_token}/>
							<input type="hidden" name="token_id" prop:value={token_id}/>

							<Link
								should_submit={true}
								class="btn-error"
								style_variant={LinkStyleVariant::Contained}
							>
								"REVOKE TOKEN"
							</Link>
						</ActionForm>
					</div>
				</div>
			</Modal>
		</Show>

		<button
			on:click={move |_| show_revoke_modal.update(|v| *v = !*v)}
			class="btn btn-error"
		>
			"REVOKE TOKEN"
		</button>
	}
}

#[component]
pub fn RegenerateApiToken() -> impl IntoView {
	let regenerate_api_token_action = create_server_action::<RegenerateApiTokenFn>();
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let params = use_params::<TokenParams>();
	let token_id = create_rw_signal(params.with(|params| {
		params
			.as_ref()
			.map(|param| param.token_id.clone().unwrap_or_default())
			.unwrap_or_default()
	}));

	let response = regenerate_api_token_action.value();

	view! {
		{
			move || match response.get() {
				Some(data) => match data {
					Ok(data) => {
						logging::log!("logging response get {:#?}", data);
						view! {
							<TokenModal is_regenerated={true} token={data.token}/>
						}.into_view()
					},
					Err(_) => view! {}.into_view()
				},
				None => view! {}.into_view()
			}
		}
		<ActionForm action={regenerate_api_token_action}>
			<input type="hidden" name="access_token" prop:value={access_token}/>
			<input type="hidden" name="token_id" prop:value={token_id}/>

			<Link
				should_submit={true}
				style_variant={LinkStyleVariant::Contained}
				class="ml-auto"
			>
				"REGENERATE TOKEN"
			</Link>
		</ActionForm>
	}
}
