use std::io;

use tokio::task;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::Instrument;

#[derive(Debug, Default, Clone)]
pub(crate) struct TaskManager
{
	#[debug("{}", tasks.len())]
	tasks: TaskTracker,

	#[debug(skip)]
	cancellation_token: CancellationToken,
}

impl TaskManager
{
	pub(crate) fn cancellation_token(&self) -> CancellationToken
	{
		self.cancellation_token.child_token()
	}

	pub(crate) fn spawn<F>(
		&self,
		span: tracing::Span,
		make_task: impl FnOnce(CancellationToken) -> F,
	) -> io::Result<task::JoinHandle<F::Output>>
	where
		F: IntoFuture,
		F::Output: Send + 'static,
		F::IntoFuture: Send + 'static,
	{
		if self.tasks.is_closed() {
			return Err(io::Error::other("task tracker has been closed"));
		}

		let current_span = tracing::Span::current();

		if !current_span.is_disabled() {
			span.follows_from(current_span);
		}

		let task_builder = task::Builder::default();
		let task_builder = if let Some(metadata) = span.metadata() {
			task_builder.name(metadata.name())
		} else {
			task_builder
		};

		task_builder.spawn({
			self.tasks.track_future({
				make_task(self.cancellation_token.child_token())
					.into_future()
					.instrument(span)
			})
		})
	}

	#[tracing::instrument(level = "debug")]
	pub async fn shutdown(self)
	{
		self.tasks.close();
		tracing::trace!("closed task tracker");

		self.cancellation_token.cancel();
		tracing::trace!("cancelled tasks");

		self.tasks.wait().await;
		tracing::trace!("all tasks have exited");
	}
}
