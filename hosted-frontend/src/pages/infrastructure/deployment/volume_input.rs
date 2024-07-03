use std::collections::BTreeMap;

use ev::MouseEvent;
use models::api::workspace::deployment::DeploymentVolume;

use crate::prelude::*;

#[component]
pub fn VolumeInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = BTreeMap::new().into())]
	volumes_list: MaybeSignal<BTreeMap<Uuid, DeploymentVolume>>,
	/// On Pressing Delete Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_delete: Callback<(MouseEvent, Uuid)>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(MouseEvent, String, String)>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_volumes = store_value(volumes_list.clone());

	let vol_path = create_rw_signal("".to_string());
	let vol_size = create_rw_signal("".to_string());
	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Volumes"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || volumes_list.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fs-fs">
							<For
								each={move || store_volumes.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:child
							>
								<div class="flex full-width mb-xs">
									<div class="flex-col-5 pr-lg">
										<div class="full-width fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md txt-of-ellipsis of-hidden-40">
												{child.1.path}
											</span>
										</div>
									</div>

									<div class="flex-col-6">
										<div class="full-width fr-sb-ct px-xl py-sm bg-secondary-light br-sm">
											<span class="px-sm">{child.1.size}</span>
											<span class="px-sm">"GB"</span>
										</div>
									</div>

									<div class="flex-col-1 fr-ct-ct pl-sm">
										<button
											on:click={
												move |ev| {
													on_delete.call((ev, child.0))
												}
											}
										>
											<Icon
												icon={IconType::Trash2}
												color={Color::Error}
												size={Size::Small}
											/>
										</button>
									</div>
								</div>
							</For>
						</div>
					</div>
				</Show>

				<div class="flex full-width">
					<div class="flex-col-5 fc-fc-fs pr-lg">
						<Input
							r#type={InputType::Text}
							id="volName"
							placeholder="Enter Volume Path"
							class="full-width"
							value={Signal::derive(move || vol_path.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_path.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<Input
							r#type={InputType::Text}
							id="envValue"
							placeholder="Enter Volume Size"
							end_text={Some("GB".to_string())}
							class="full-width"
							value={Signal::derive(move || vol_size.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_size.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-1 fr-ct-fs">
						<button
							on:click={move |ev| {
								on_add.call((ev, vol_path.get(), vol_size.get()))
							}}
							class="btn btn-primary br-sm p-xs ml-md"
						>
							<Icon icon={IconType::Plus} color={Color::Secondary}/>
						</button>
					</div>
				</div>
			</div>
		</div>
	}
}
