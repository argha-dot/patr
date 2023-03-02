use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		core::v1::{Service, ServicePort, ServiceSpec},
		networking::v1::{
			HTTPIngressPath,
			HTTPIngressRuleValue,
			Ingress,
			IngressBackend,
			IngressRule,
			IngressServiceBackend,
			IngressSpec,
			IngressTLS,
			ServiceBackendPort,
		},
	},
	apimachinery::pkg::util::intstr::IntOrString,
};
use kube::{
	self,
	api::{DeleteParams, Patch, PatchParams},
	core::ObjectMeta,
	Api,
};
use kubernetes::ext_traits::DeleteOpt;

use crate::{
	db,
	error,
	service::{self, infrastructure::kubernetes},
	utils::{settings::Settings, Error},
};

pub async fn update_kubernetes_managed_url(
	workspace_id: &Uuid,
	managed_url: &ManagedUrl,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - generating managed url configuration",
		request_id
	);

	let domain = db::get_workspace_domain_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&managed_url.domain_id,
	)
	.await?
	.status(500)?;

	let secret_name = if domain.is_ns_internal() {
		format!("tls-{}", domain.id)
	} else {
		format!("tls-{}-{}", managed_url.sub_domain, managed_url.id)
	};

	let secret_exists = kubernetes::secret_exists(
		&secret_name,
		kubernetes_client.clone(),
		namespace,
	)
	.await?;

	let host = if managed_url.sub_domain == "@" {
		domain.name.clone()
	} else {
		format!("{}.{}", managed_url.sub_domain, domain.name)
	};

	let (ingress, annotations) = match &managed_url.url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => (
			IngressRule {
				host: Some(host.clone()),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-{}", deployment_id),
								port: Some(ServiceBackendPort {
									number: Some(*port as i32),
									..ServiceBackendPort::default()
								}),
							}),
							..Default::default()
						},
						path: Some(managed_url.path.to_string()),
						path_type: "Prefix".to_string(),
					}],
				}),
			},
			[
				(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				),
				(
					"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
					host.clone(),
				),
				(
					"cert-manager.io/cluster-issuer".to_string(),
					if domain.is_ns_internal() {
						config.kubernetes.cert_issuer_dns.clone()
					} else {
						config.kubernetes.cert_issuer_http.clone()
					},
				),
			]
			.into_iter()
			.collect(),
		),
		ManagedUrlType::ProxyStaticSite { static_site_id } => {
			let static_site = db::get_static_site_by_id(
				service::get_app().database.acquire().await?.deref_mut(),
				static_site_id,
			)
			.await?
			.status(500)?;
			(
				IngressRule {
					host: Some(host.clone()),
					http: Some(HTTPIngressRuleValue {
						paths: vec![HTTPIngressPath {
							backend: IngressBackend {
								service: Some(IngressServiceBackend {
									name: format!("service-{}", static_site_id),
									port: Some(ServiceBackendPort {
										number: Some(80),
										..ServiceBackendPort::default()
									}),
								}),
								..Default::default()
							},
							path: Some(managed_url.path.to_string()),
							path_type: "Prefix".to_string(),
						}],
					}),
				},
				[
					(
						"kubernetes.io/ingress.class".to_string(),
						"nginx".to_string(),
					),
					(
						"nginx.ingress.kubernetes.io/upstream-vhost"
							.to_string(),
						if let Some(upload_id) = static_site.current_live_upload
						{
							format!(
								"{}-{}.patr.cloud",
								upload_id, static_site_id
							)
						} else {
							format!("{}.patr.cloud", static_site_id)
						},
					),
					(
						"cert-manager.io/cluster-issuer".to_string(),
						if domain.is_ns_internal() {
							config.kubernetes.cert_issuer_dns.clone()
						} else {
							config.kubernetes.cert_issuer_http.clone()
						},
					),
				]
				.into_iter()
				.collect(),
			)
		}
		ManagedUrlType::ProxyUrl { url, http_only } => {
			let kubernetes_service = Service {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", managed_url.id)),
					..ObjectMeta::default()
				},
				spec: Some(ServiceSpec {
					type_: Some("ExternalName".to_string()),
					external_name: Some(url.clone()),
					ports: Some(vec![ServicePort {
						name: Some(
							if *http_only {
								"http".to_string()
							} else {
								"https".to_string()
							},
						),
						port: if *http_only { 80 } else { 443 },
						protocol: Some("TCP".to_string()),
						target_port: Some(IntOrString::Int(
							if *http_only { 80 } else { 443 },
						)),
						..ServicePort::default()
					}]),
					..ServiceSpec::default()
				}),
				..Service::default()
			};
			// Create the service defined above
			log::trace!(
				"request_id: {} - creating ExternalName service",
				request_id
			);
			let service_api: Api<Service> =
				Api::namespaced(kubernetes_client.clone(), namespace);
			service_api
				.patch(
					&format!("service-{}", managed_url.id),
					&PatchParams::apply(&format!("service-{}", managed_url.id)),
					&Patch::Apply(kubernetes_service),
				)
				.await?
				.status
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

			(
				IngressRule {
					host: Some(host.clone()),
					http: Some(HTTPIngressRuleValue {
						paths: vec![HTTPIngressPath {
							backend: IngressBackend {
								service: Some(IngressServiceBackend {
									name: format!("service-{}", managed_url.id),
									port: Some(ServiceBackendPort {
										number: Some(
											if *http_only { 80 } else { 443 },
										),
										..ServiceBackendPort::default()
									}),
								}),
								..Default::default()
							},
							path: Some(managed_url.path.to_string()),
							path_type: "Prefix".to_string(),
						}],
					}),
				},
				[
					(
						"kubernetes.io/ingress.class".to_string(),
						"nginx".to_string(),
					),
					(
						String::from(
							"nginx.ingress.kubernetes.io/upstream-vhost",
						),
						url.clone(),
					),
					(
						String::from(
							"nginx.ingress.kubernetes.io/backend-protocol",
						),
						if *http_only {
							"HTTP".to_string()
						} else {
							"HTTPS".to_string()
						},
					),
					(
						"cert-manager.io/cluster-issuer".to_string(),
						if domain.is_ns_internal() {
							config.kubernetes.cert_issuer_dns.clone()
						} else {
							config.kubernetes.cert_issuer_http.clone()
						},
					),
				]
				.into_iter()
				.collect(),
			)
		}
		ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => {
			let kubernetes_service = Service {
				metadata: ObjectMeta {
					name: Some(format!("service-{}", managed_url.id)),
					..ObjectMeta::default()
				},
				spec: Some(ServiceSpec {
					type_: Some("ExternalName".to_string()),
					external_name: Some(url.clone()),
					ports: Some(vec![ServicePort {
						name: Some(
							if *http_only {
								"http".to_string()
							} else {
								"https".to_string()
							},
						),
						port: if *http_only { 80 } else { 443 },
						protocol: Some("TCP".to_string()),
						target_port: Some(IntOrString::Int(
							if *http_only { 80 } else { 443 },
						)),
						..ServicePort::default()
					}]),
					..ServiceSpec::default()
				}),
				..Service::default()
			};
			// Create the service defined above
			log::trace!(
				"request_id: {} - creating ExternalName service",
				request_id
			);
			let service_api = Api::<Service>::namespaced(
				kubernetes_client.clone(),
				namespace,
			);
			service_api
				.patch(
					&format!("service-{}", managed_url.id),
					&PatchParams::apply(&format!("service-{}", managed_url.id)),
					&Patch::Apply(kubernetes_service),
				)
				.await?
				.status
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
			(
				IngressRule {
					host: Some(host.clone()),
					http: Some(HTTPIngressRuleValue {
						paths: vec![HTTPIngressPath {
							backend: IngressBackend {
								service: Some(IngressServiceBackend {
									name: format!("service-{}", managed_url.id),
									port: Some(ServiceBackendPort {
										number: Some(
											if *http_only { 80 } else { 443 },
										),
										..ServiceBackendPort::default()
									}),
								}),
								..Default::default()
							},
							path: Some(managed_url.path.to_string()),
							path_type: "Prefix".to_string(),
						}],
					}),
				},
				[
					(
						"kubernetes.io/ingress.class".to_string(),
						"nginx".to_string(),
					),
					if *permanent_redirect {
						(
							"nginx.ingress.kubernetes.io/permanent-redirect"
								.to_string(),
							if *http_only {
								format!("http://{}", url)
							} else {
								format!("https://{}", url)
							},
						)
					} else {
						(
							"nginx.ingress.kubernetes.io/temporal-redirect"
								.to_string(),
							if *http_only {
								format!("http://{}", url)
							} else {
								format!("https://{}", url)
							},
						)
					},
					(
						"cert-manager.io/cluster-issuer".to_string(),
						if domain.is_ns_internal() {
							config.kubernetes.cert_issuer_dns.clone()
						} else {
							config.kubernetes.cert_issuer_http.clone()
						},
					),
				]
				.into_iter()
				.collect(),
			)
		}
	};

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", managed_url.id)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: if secret_exists {
				Some(vec![IngressTLS {
					hosts: if domain.is_ns_internal() {
						Some(vec![
							format!("*.{}", domain.name),
							domain.name.clone(),
						])
					} else {
						Some(vec![host.clone()])
					},
					secret_name: Some(secret_name),
				}])
			} else {
				None
			},
			..IngressSpec::default()
		}),
		..Ingress::default()
	};
	// Create the ingress defined above
	log::trace!("request_id: {} - creating ingress", request_id);
	Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.patch(
			&format!("ingress-{}", managed_url.id),
			&PatchParams::apply(&format!("ingress-{}", managed_url.id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("request_id: {} - managed URL created", request_id);
	Ok(())
}

pub async fn delete_kubernetes_managed_url(
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}",
		request_id,
		managed_url_id
	);

	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(
			&format!("service-{}", managed_url_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - deleting ingress {}",
		request_id,
		managed_url_id
	);

	Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.delete_opt(
			&format!("ingress-{}", managed_url_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - managed URL deleted successfully!",
		request_id
	);
	Ok(())
}

pub async fn create_managed_url_verification_ingress(
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	sub_domain: &str,
	domain_name: &str,
	verification_token: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;
	let namespace = workspace_id.as_str();

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}-verification", managed_url_id)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			type_: Some("ExternalName".to_string()),
			external_name: Some("api.patr.cloud".to_string()),
			ports: Some(vec![
				ServicePort {
					name: Some("https".to_string()),
					port: 443,
					protocol: Some("TCP".to_string()),
					target_port: Some(IntOrString::Int(443)),
					..ServicePort::default()
				},
				ServicePort {
					name: Some("http".to_string()),
					port: 80,
					protocol: Some("TCP".to_string()),
					target_port: Some(IntOrString::Int(80)),
					..ServicePort::default()
				},
			]),
			..ServiceSpec::default()
		}),
		..Service::default()
	};

	// Create the service defined above
	log::trace!(
		"request_id: {} - creating ExternalName verification service",
		request_id
	);
	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.patch(
			&format!("service-{}-verification", managed_url_id),
			&PatchParams::apply(&format!(
				"service-{}-verification",
				managed_url_id
			)),
			&Patch::Apply(kubernetes_service),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let ingress = IngressRule {
		host: Some(
			if sub_domain == "@" {
				domain_name.to_string()
			} else {
				format!("{}.{}", sub_domain, domain_name)
			},
		),
		http: Some(HTTPIngressRuleValue {
			paths: vec![HTTPIngressPath {
				backend: IngressBackend {
					service: Some(IngressServiceBackend {
						name: format!(
							"service-{}-verification",
							managed_url_id
						),
						port: Some(ServiceBackendPort {
							number: Some(80),
							..ServiceBackendPort::default()
						}),
					}),
					..Default::default()
				},
				path: Some("/.well-known/patr-verification/".to_string()),
				path_type: "Prefix".to_string(),
			}],
		}),
	};

	let kubernetes_ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}-verification", managed_url_id)),
			annotations: Some(
				[
					(
						"kubernetes.io/ingress.class".to_string(),
						"nginx".to_string(),
					),
					(
						String::from(
							"nginx.ingress.kubernetes.io/temporal-redirect",
						),
						format!(
							"{}{}{}{}/verification/{}",
							"http://api.patr.cloud/workspace/",
							workspace_id,
							"/infrastructure/managed-url/",
							managed_url_id,
							verification_token
						),
					),
				]
				.into_iter()
				.collect(),
			),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![ingress]),
			tls: None,
			..IngressSpec::default()
		}),
		..Ingress::default()
	};
	// Create the ingress defined above
	log::trace!(
		"request_id: {} - creating verification string ingress",
		request_id
	);
	Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.patch(
			&format!("ingress-{}-verification", managed_url_id),
			&PatchParams::apply(&format!(
				"ingress-{}-verification",
				managed_url_id
			)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?;

	Ok(())
}

pub async fn delete_kubernetes_managed_url_verification(
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client = super::get_kubernetes_config(config).await?;

	let namespace = workspace_id.as_str();
	log::trace!(
		"request_id: {} - deleting service: service-{}-verification",
		request_id,
		managed_url_id
	);

	Api::<Service>::namespaced(kubernetes_client.clone(), namespace)
		.delete_opt(
			&format!("service-{}-verification", managed_url_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - deleting ingress {}",
		request_id,
		managed_url_id
	);

	Api::<Ingress>::namespaced(kubernetes_client, namespace)
		.delete_opt(
			&format!("ingress-{}-verification", managed_url_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - managed URL deleted successfully!",
		request_id
	);
	Ok(())
}
