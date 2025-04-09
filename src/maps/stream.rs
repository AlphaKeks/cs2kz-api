use super::Map;

pub(super) struct StreamState<S>
{
	stream: S,
	curr: Option<Map>,
}

impl<S> StreamState<S>
{
	pub(super) fn new(stream: S) -> Self
	{
		Self { stream, curr: None }
	}
}

pub(super) macro from_raw($stream: expr) {{
	use {
		super::{CS2Filters, CSGOFilters, Filter},
		crate::mode::Mode,
		std::{collections::btree_map, mem},
	};

	futures_util::stream::unfold(StreamState::new($stream), async |mut state| {
		loop {
			let Some(curr) = state.curr.as_mut() else {
				match state.stream.next().await? {
					Ok(row) => {
						state.curr = Some(parse_row!(row));
						continue;
					},
					Err(err) => return Some((Err(err), state)),
				}
			};

			let next_row = match state.stream.next().await {
				None => {
					let map = state.curr.take().unwrap_or_else(|| {
						panic!("`curr` exists so `state.curr` must be `Some`");
					});

					return Some((Ok(map), state));
				},
				Some(Ok(row)) => row,
				Some(Err(err)) => return Some((Err(err), state)),
			};

			let next = parse_row!(next_row);

			if next.id != curr.id {
				return Some((Ok(mem::replace(curr, next)), state));
			}

			for (course_id, mut course) in next.courses {
				match curr.courses.entry(course_id) {
					btree_map::Entry::Vacant(entry) => {
						entry.insert(course);
					},
					btree_map::Entry::Occupied(mut entry) => {
						entry.get_mut().mappers.append(&mut course.mappers);

						let old_filters = &mut entry.get_mut().filters;
						let filter = || Filter {
							id: next_row.filter_id,
							nub_tier: next_row.filter_nub_tier,
							pro_tier: next_row.filter_pro_tier,
							ranked: next_row.filter_ranked,
							notes: next_row.filter_notes.clone(),
						};

						match next_row.filter_mode {
							Mode::KZTimer => {
								if let Some(ref mut filters) = old_filters.csgo {
									filters.kzt = filter();
								} else {
									old_filters.csgo = Some(CSGOFilters {
										kzt: filter(),
										skz: filter(),
										vnl: filter(),
									});
								}
							},
							Mode::SimpleKZ => {
								if let Some(ref mut filters) = old_filters.csgo {
									filters.skz = filter();
								} else {
									old_filters.csgo = Some(CSGOFilters {
										kzt: filter(),
										skz: filter(),
										vnl: filter(),
									});
								}
							},
							Mode::VanillaCSGO => {
								if let Some(ref mut filters) = old_filters.csgo {
									filters.vnl = filter();
								} else {
									old_filters.csgo = Some(CSGOFilters {
										kzt: filter(),
										skz: filter(),
										vnl: filter(),
									});
								}
							},
							Mode::VanillaCS2 => {
								if let Some(ref mut filters) = old_filters.cs2 {
									filters.vnl = filter();
								} else {
									old_filters.cs2 =
										Some(CS2Filters { vnl: filter(), ckz: filter() });
								}
							},
							Mode::Classic => {
								if let Some(ref mut filters) = old_filters.cs2 {
									filters.ckz = filter();
								} else {
									old_filters.cs2 =
										Some(CS2Filters { vnl: filter(), ckz: filter() });
								}
							},
						}
					},
				}
			}
		}
	})
}}

macro parse_row($row:expr) {{
	use {
		super::{CS2Filters, CSGOFilters, Course, Filter, Filters, Mapper},
		crate::mode::Mode,
		std::collections::BTreeMap,
	};

	Map {
		id: $row.id,
		workshop_id: $row.workshop_id,
		name: $row.name,
		description: $row.description,
		game: $row.game,
		state: $row.state,
		checksum: $row.checksum,
		courses: BTreeMap::from_iter([($row.course_id, Course {
			id: $row.course_id,
			local_id: $row.course_local_id,
			name: $row.course_name,
			description: $row.course_description,
			mappers: BTreeMap::from_iter([($row.course_mapper_id, Mapper {
				id: $row.course_mapper_id,
				name: $row.course_mapper_name,
			})]),
			filters: {
				let filter = || Filter {
					id: $row.filter_id,
					nub_tier: $row.filter_nub_tier,
					pro_tier: $row.filter_pro_tier,
					ranked: $row.filter_ranked,
					notes: $row.filter_notes.clone(),
				};

				match $row.filter_mode {
					Mode::VanillaCS2 | Mode::Classic => Filters {
						cs2: Some(CS2Filters { vnl: filter(), ckz: filter() }),
						csgo: None,
					},
					Mode::KZTimer | Mode::SimpleKZ | Mode::VanillaCSGO => Filters {
						cs2: None,
						csgo: Some(CSGOFilters { kzt: filter(), skz: filter(), vnl: filter() }),
					},
				}
			},
		})]),
		created_by: Mapper { id: $row.mapper_id, name: $row.mapper_name },
		created_at: $row.created_at,
	}
}}
