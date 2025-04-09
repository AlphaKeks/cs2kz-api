use {
	futures_util::{Stream, TryStream, TryStreamExt as _, stream::FusedStream},
	std::{
		fmt,
		future,
		pin::Pin,
		task::{self, Poll, ready},
	},
};

pub trait StreamExt: Stream
{
	fn instrumented(self, span: tracing::Span) -> Instrumented<Self>
	where
		Self: Sized + FusedStream,
		Self::Item: fmt::Debug,
	{
		Instrumented { stream: self, span }
	}
}

impl<S: Stream> StreamExt for S
{
}

pub trait TryStreamExt: TryStream
{
	fn try_collect_into<C>(
		self,
		collection: &mut C,
	) -> impl Future<Output = Result<&mut C, Self::Error>>
	where
		Self: Sized,
		C: Extend<Self::Ok>,
	{
		collection.extend_reserve(self.size_hint().0);

		self.try_fold(collection, |collection, item| {
			collection.extend_one(item);
			future::ready(Ok(collection))
		})
	}
}

impl<S: TryStream> TryStreamExt for S
{
}

#[pin_project(project = InstrumentedProj)]
#[derive(Debug)]
pub struct Instrumented<S>
where
	S: FusedStream<Item: fmt::Debug>,
{
	#[pin]
	stream: S,
	span: tracing::Span,
}

impl<S> Stream for Instrumented<S>
where
	S: FusedStream<Item: fmt::Debug>,
{
	type Item = S::Item;

	fn poll_next(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>>
	{
		let InstrumentedProj { stream, span } = self.project();

		span.in_scope(|| {
			Poll::Ready(match ready!(stream.poll_next(cx)) {
				Some(item) => {
					trace!(?item);
					Some(item)
				},
				None => {
					trace!("stream is exhausted");
					None
				},
			})
		})
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		self.stream.size_hint()
	}
}

impl<S> FusedStream for Instrumented<S>
where
	S: FusedStream<Item: fmt::Debug>,
{
	fn is_terminated(&self) -> bool
	{
		self.stream.is_terminated()
	}
}
