use problem_details::AsProblemDetails;
use serde::{Serialize, Serializer};
use tokio::{runtime, task};

#[derive(Debug)]
pub struct Taskdump<'a>
{
	task_id: task::Id,
	trace: &'a runtime::dump::Trace,
}

impl<'a> Taskdump<'a>
{
	pub(super) fn new(task: &'a runtime::dump::Task) -> Self
	{
		Self {
			task_id: task.id(),
			trace: task.trace(),
		}
	}
}

impl Serialize for Taskdump<'_>
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		use serde::ser::SerializeMap;

		let mut serializer = serializer.serialize_map(Some(2))?;
		let task_id = format!("{:?}", self.task_id);
		let task_id = task_id
			.trim_matches(|c: char| !c.is_numeric())
			.parse::<u64>()
			.unwrap();

		serializer.serialize_entry("task_id", &task_id)?;
		serializer.serialize_entry("trace", &format_args!("{}", self.trace))?;
		serializer.end()
	}
}

#[derive(Debug, Error)]
pub enum Error
{
	#[error("dump timed out")]
	Timeout(#[from] tokio::time::error::Elapsed),

	#[error("failed to serialize dump: {0}")]
	SerializeDump(#[from] serde_json::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = crate::http::Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		<Self::ProblemType>::Internal
	}
}

impl_into_response!(Error);
