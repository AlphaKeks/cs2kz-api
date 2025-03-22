use std::{sync::LazyLock, thread};

use futures_util::TryFutureExt;
use pyo3::Python;
use tokio::sync::{mpsc, oneshot};

enum Message
{
	Job(Box<dyn for<'a, 'py> FnOnce(&'a PyState<'py>) + Send + 'static>),
	Shutdown(oneshot::Sender<()>),
}

static PYTHON_THREAD: LazyLock<mpsc::Sender<Message>> = LazyLock::new(|| {
	let (tx, mut rx) = mpsc::channel(64);

	thread::spawn(move || {
		let _guard = tracing::info_span!("pyo3").entered();

		Python::with_gil(|python| {
			let state = PyState { python };

			tracing::info!("waiting for jobs");
			while let Some(message) = rx.blocking_recv() {
				match message {
					Message::Job(job) => job(&state),
					Message::Shutdown(sender) => {
						tracing::warn!("shutting down");
						let _ = sender.send(());
						break;
					},
				}
			}
		})
	});

	tx
});

#[derive(Debug)]
pub struct PyState<'py>
{
	#[debug(skip)]
	python: Python<'py>,
}

impl<'py> PyState<'py>
{
	pub fn python(&self) -> Python<'py>
	{
		self.python
	}
}

#[derive(Debug, Display, Error)]
pub enum PythonError
{
	#[display("python thread has already shut down")]
	Shutdown,

	#[display("python thread has panicked")]
	Panic(oneshot::error::RecvError),
}

pub async fn execute<T>(
	job: impl for<'a, 'py> FnOnce(&'a PyState<'py>) -> T + Send + 'static,
) -> Result<T, PythonError>
where
	T: Send + 'static,
{
	let (tx, rx) = oneshot::channel();

	PYTHON_THREAD
		.send(Message::Job(Box::new(move |state| {
			let _ = tx.send(job(state));
		})))
		.map_err(|_| PythonError::Shutdown)
		.and_then(|()| rx.map_err(PythonError::Panic))
		.await
}

#[tracing::instrument(err)]
pub async fn shutdown() -> Result<(), PythonError>
{
	let (tx, rx) = oneshot::channel();

	PYTHON_THREAD
		.send(Message::Shutdown(tx))
		.map_err(|_| PythonError::Shutdown)
		.and_then(|()| rx.map_err(PythonError::Panic))
		.await
}
