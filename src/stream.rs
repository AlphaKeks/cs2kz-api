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
		Self::Item: fmt::Debug;
}

impl<S: Stream> StreamExt for S
{
	fn instrumented(self, span: tracing::Span) -> Instrumented<Self>
	where
		Self: Sized + FusedStream,
		Self::Item: fmt::Debug,
	{
		Instrumented { stream: self, span }
	}
}

pub trait TryStreamExt: TryStream
{
	/// Collects all remaining items into the given `collection` but
	/// short-circuits on the first error (if any).
	async fn try_collect_into<C>(self, collection: &mut C) -> Result<&mut C, Self::Error>
	where
		Self: Sized,
		C: Extend<Self::Ok>;
}

impl<S: TryStream> TryStreamExt for S
{
	async fn try_collect_into<C>(self, collection: &mut C) -> Result<&mut C, Self::Error>
	where
		Self: Sized,
		C: Extend<Self::Ok>,
	{
		collection.extend_reserve(self.size_hint().0);

		self.try_fold(collection, |collection, item| {
			collection.extend_one(item);
			future::ready(Ok(collection))
		})
		.await
	}
}

#[pin_project]
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
		let me = self.project();
		let _guard = me.span.enter();
		let maybe_item = ready!(me.stream.poll_next(cx));

		if let Some(ref item) = maybe_item {
			trace!(?item);
		} else {
			trace!("stream is exhausted");
		}

		Poll::Ready(maybe_item)
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
