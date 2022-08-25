use std::{collections::BTreeMap, fmt::Display, ops::Deref};

use monostate::MustBe;
use serde::{Deserialize, Serialize};

/// Indicates the type of task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind")]
pub enum CiFlow {
	/// CI pipeline task
	Pipeline(Pipeline),
}

/// Pipeline action defines the CI pipeline steps which will be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
	/// version of pipeline
	pub version: MustBe!("v1"),
	/// name of pipeline
	pub name: LabelName,
	/// list of services to run in background while executing pipeline
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub services: Vec<Service>,
	/// list of steps to be executed in a single pipeline
	pub steps: Vec<Step>,
}

/// Step represent a single unit of work or a decision block which will be done
/// in pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
	Work(Work),
	Decision(Decision),
}

/// A decision block decides the next steps based on the branches and events
/// during CI initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
	/// name of the decision
	pub name: LabelName,
	/// a condition which will return either true or false based on the brach
	/// and events
	pub when: When,
	/// if condition is evaluated to true, then clause will be executed next
	pub then: LabelName,
	/// an optional else case which will be executed next, if the condition
	/// evaluates to false
	#[serde(alias = "else")]
	pub else_: Option<LabelName>,
}

/// A decision block decides the next steps based on the branches and events
/// during CI initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
	/// Represents the list of branch in glob pattern which will be matched
	pub branch: OneOrMany<String>,
}

/// Work represent a single unit of work which will be done in pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Work {
	/// name of the work
	pub name: LabelName,
	/// image can be any docker image hosted publicly on docker hub
	pub image: String,
	/// list of commands to be executed with the given image.
	/// these commands will start executing from the repo source
	pub command: OneOrMany<String>,
	/// list of environmental variables that has to be defined while
	/// initializing container
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub env: BTreeMap<String, EnvVarValue>,
}

/// Service represents a background job which will run during pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
	/// name of the service
	pub name: LabelName,
	/// image can be any docker image hosted publicly on docker hub
	pub image: String,
	/// list of commands to be executed with the given image.
	/// these commands will start executing from the repo source
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub command: Option<OneOrMany<String>>,
	/// list of environmental variables that has to be defined while
	/// initializing container
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub env: BTreeMap<String, EnvVarValue>,
	/// TCP port to access this service
	pub port: i32,
}

/// A decorative wrapper to use either a one or many values of same type
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
	/// A single value
	One(T),
	/// Multiple values
	Many(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
	fn from(from: OneOrMany<T>) -> Self {
		match from {
			OneOrMany::One(value) => vec![value],
			OneOrMany::Many(values) => values,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum EnvVarValue {
	Value(String),
	ValueFromSecret { from_secret: String },
}

/// A wrapped string type used to represent the valid naming
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct LabelName(String);

impl LabelName {
	pub fn is_valid_label_name(name: &str) -> bool {
		// https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#dns-subdomain-names
		// 1. 0 < label.len() <= 63
		// 2. contain only lowercase alphanumeric characters or '-'
		// 3. starts and ends with an alphanumeric character

		!name.is_empty() &&
			name.len() <= 63 &&
			name.bytes()
				.all(|ch| matches!(ch, b'a'..=b'z' | b'0'..=b'9' | b'-')) &&
			!name.starts_with('-') &&
			!name.ends_with('-')
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<LabelName> for String {
	fn from(label: LabelName) -> Self {
		label.0
	}
}

impl TryFrom<String> for LabelName {
	type Error = String;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		if Self::is_valid_label_name(&value) {
			Ok(Self(value))
		} else {
			Err(format!("Invalid label value: {value}"))
		}
	}
}

impl Deref for LabelName {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Display for LabelName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
