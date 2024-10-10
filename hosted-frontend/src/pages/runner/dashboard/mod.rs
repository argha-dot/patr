mod head;
mod runner_card;

use leptos_query::QueryResult;

pub use self::{head::*, runner_card::*};
use crate::{prelude::*, queries::*};

/// The Runner Dashboard page
#[component]
pub fn RunnerDashboard() -> impl IntoView {
	let QueryResult {
		data: runners_list, ..
	} = list_runners_query().use_query(move || AllRunnersTag);

	view! {
		<RunnerDashboardHead />
		<ContainerBody class="p-xs gap-md">
			<DashboardContainer
				gap={Size::Large}
				render_items={view! {
					<Transition>
						{move || match runners_list.get() {
							Some(Ok(data)) => {
								view! {
									<For
										each={move || data.runners.clone()}
										key={|state| state.id}
										let:runner
									>
										<RunnerCard runner={runner} />
									</For>
								}
									.into_view()
							}
							Some(Err(_)) => view! {}.into_view(),
							None => view! { <RunnerCardSkeleton /> }.into_view(),
						}}
					</Transition>
				}
					.into_view()}
			/>
		</ContainerBody>
	}
}
