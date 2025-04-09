//! A [`tracing_subscriber::Layer`] implementation for recording events as
//! Discord embeds and sending them over a channel to a [`Bot`]
//!
//! [`Bot`]: super::Bot

use {
	crate::time::Timestamp,
	poise::serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedFooter},
	std::{
		collections::HashMap,
		error::Error,
		fmt::{self, Write},
		mem,
	},
	tokio::sync::mpsc,
	tracing::{field::Field, span},
	tracing_subscriber::{field, layer, registry},
};

#[derive(Debug)]
pub struct Layer
{
	tx: mpsc::WeakSender<CreateEmbed>,
}

struct EventVisitor
{
	embed: CreateEmbed,
}

#[derive(Debug)]
struct Fields(HashMap<&'static str, String>);

impl Layer
{
	pub(super) fn new(tx: &mpsc::Sender<CreateEmbed>) -> Self
	{
		Self { tx: tx.downgrade() }
	}
}

impl<S> tracing_subscriber::Layer<S> for Layer
where
	S: tracing::Subscriber + for<'a> registry::LookupSpan<'a>,
{
	fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>)
	{
		let span = ctx.span(id).unwrap_or_else(|| {
			panic!("received invalid span id from subscriber");
		});

		let mut fields = Fields(HashMap::with_capacity(attrs.values().len()));
		attrs.record(&mut fields);
		span.extensions_mut().insert(fields);
	}

	fn on_record(&self, span_id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>)
	{
		let span = ctx.span(span_id).unwrap_or_else(|| {
			panic!("received invalid span id from subscriber");
		});

		if let Some(fields) = span.extensions_mut().get_mut::<Fields>() {
			values.record(fields);
		}
	}

	fn on_event(&self, event: &tracing::Event<'_>, ctx: layer::Context<'_, S>)
	{
		if *event.metadata().level() > tracing::Level::WARN {
			return;
		}

		let Some(tx) = self.tx.upgrade() else {
			return;
		};

		if let Ok(permit) = tx.try_reserve() {
			let mut visitor = EventVisitor::new(event, ctx.event_span(event));
			event.record(&mut visitor);
			permit.send(visitor.into_embed());
		}
	}
}

impl EventVisitor
{
	fn new<'r, R>(event: &tracing::Event<'_>, parent_span: Option<registry::SpanRef<'r, R>>) -> Self
	where
		R: registry::LookupSpan<'r>,
	{
		let (colour, level) = match *event.metadata().level() {
			tracing::Level::TRACE => (serenity::Colour::TEAL, "TRACE"),
			tracing::Level::DEBUG => (serenity::Colour::BLUE, "DEBUG"),
			tracing::Level::INFO => (serenity::Colour::DARK_GREEN, "INFO"),
			tracing::Level::WARN => (serenity::Colour::ORANGE, "WARN"),
			tracing::Level::ERROR => (serenity::Colour::RED, "ERROR"),
		};

		let title = format!("({}) `{}`", level, event.metadata().target());
		let location = format!(
			"`{}:{}`",
			event
				.metadata()
				.file()
				.or_else(|| event.metadata().module_path())
				.unwrap_or("<unknown>"),
			event.metadata().line().unwrap_or_default(),
		);

		let footer = CreateEmbedFooter::new(Timestamp::now().to_string());

		let mut embed = CreateEmbed::default()
			.colour(colour)
			.title(title)
			.field("Location", location, true)
			.footer(footer);

		if let Some(span) = parent_span {
			let extensions = span.extensions();

			if let Some(fields) = extensions.get::<Fields>().map(|&Fields(ref raw_fields)| {
				raw_fields.iter().map(|(&name, value)| (name, value.clone(), false))
			}) {
				embed = embed.fields(fields);
			}
		}

		Self { embed }
	}

	fn into_embed(self) -> CreateEmbed
	{
		self.embed
	}
}

impl field::Visit for EventVisitor
{
	fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug)
	{
		let name = field.name();
		let mut value = format!("{value:?}");

		if name == "message" {
			if value.starts_with('"') {
				value.remove(0);
			}

			if value.ends_with('"') {
				value.pop();
			}

			self.embed = mem::take(&mut self.embed).description(format!("```\n{value}\n```"));
		} else {
			/// Field size limit imposed by Discord
			const LIMIT: usize = 1024;

			let cutoff = value.floor_char_boundary(const { LIMIT - "``".len() - "...".len() });
			let mut formatted_value = format!("`{}", &value[..cutoff]);

			if cutoff < (value.len() - 1) {
				formatted_value.push_str("...");
			}

			formatted_value.push('`');

			self.embed = mem::take(&mut self.embed).field(name, formatted_value, false);
		}
	}

	fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static))
	{
		// TODO: capture error sources?
		self.embed = mem::take(&mut self.embed).field(field.name(), value.to_string(), false);
	}

	fn record_str(&mut self, field: &Field, value: &str)
	{
		self.embed = mem::take(&mut self.embed).field(field.name(), format!("{value:?}"), false);
	}
}

impl field::Visit for Fields
{
	fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug)
	{
		let _ = write!(self.0.entry(field.name()).or_default(), "`{value:?}`");
	}

	fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static))
	{
		// TODO: capture error sources?
		let _ = write!(self.0.entry(field.name()).or_default(), "{value}");
	}

	fn record_str(&mut self, field: &Field, value: &str)
	{
		let _ = write!(
			self.0
				.entry(field.name())
				.or_insert_with(|| String::with_capacity(value.len())),
			"{value:?}"
		);
	}
}
