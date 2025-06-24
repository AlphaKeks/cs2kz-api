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
				return Some((Ok(std::mem::replace(curr, next)), state));
			}

			for (course_id, mut course) in next.courses {
				match curr.courses.entry(course_id) {
					std::collections::btree_map::Entry::Vacant(entry) => {
						entry.insert(course);
					},
					std::collections::btree_map::Entry::Occupied(mut entry) => {
						entry.get_mut().mappers.append(&mut course.mappers);

						let filter = || $crate::maps::Filter {
							id: next_row.filter_id,
							nub_tier: next_row.filter_nub_tier,
							pro_tier: next_row.filter_pro_tier,
							ranked: next_row.filter_ranked,
							notes: next_row.filter_notes.clone(),
						};

						match (next_row.filter_mode, &mut entry.get_mut().filters) {
							(
								$crate::mode::Mode::VanillaCS2,
								$crate::maps::Filters::CS2 { vnl, .. },
							) => {
								*vnl = filter();
							},
							(
								$crate::mode::Mode::Classic,
								$crate::maps::Filters::CS2 { ckz, .. },
							) => {
								*ckz = filter();
							},
							(
								$crate::mode::Mode::KZTimer,
								$crate::maps::Filters::CSGO { kzt, .. },
							) => {
								*kzt = filter();
							},
							(
								$crate::mode::Mode::SimpleKZ,
								$crate::maps::Filters::CSGO { skz, .. },
							) => {
								*skz = filter();
							},
							(
								$crate::mode::Mode::VanillaCSGO,
								$crate::maps::Filters::CSGO { vnl, .. },
							) => {
								*vnl = filter();
							},
							state => unreachable!("invalid filter state: {state:?}"),
						}
					},
				}
			}
		}
	})
}}

macro parse_row($row:expr) {
	Map {
		id: $row.id,
		workshop_id: $row.workshop_id,
		name: $row.name,
		description: $row.description,
		game: $row.game,
		state: $row.state,
		checksum: $row.checksum,
		courses: std::collections::BTreeMap::from_iter([($row.course_id, $crate::maps::Course {
			id: $row.course_id,
			local_id: $row.course_local_id,
			name: $row.course_name,
			description: $row.course_description,
			mappers: std::collections::BTreeMap::from_iter([(
				$row.course_mapper_id,
				$crate::maps::Mapper { id: $row.course_mapper_id, name: $row.course_mapper_name },
			)]),
			filters: {
				let filter = || $crate::maps::Filter {
					id: $row.filter_id,
					nub_tier: $row.filter_nub_tier,
					pro_tier: $row.filter_pro_tier,
					ranked: $row.filter_ranked,
					notes: $row.filter_notes.clone(),
				};

				match $row.filter_mode {
					$crate::mode::Mode::VanillaCS2 | $crate::mode::Mode::Classic => {
						$crate::maps::Filters::CS2 { vnl: filter(), ckz: filter() }
					},
					$crate::mode::Mode::KZTimer
					| $crate::mode::Mode::SimpleKZ
					| $crate::mode::Mode::VanillaCSGO => {
						$crate::maps::Filters::CSGO { kzt: filter(), skz: filter(), vnl: filter() }
					},
				}
			},
		})]),
		created_by: $crate::maps::Mapper { id: $row.mapper_id, name: $row.mapper_name },
		created_at: $row.created_at,
	}
}
