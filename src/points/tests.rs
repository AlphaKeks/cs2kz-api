use color_eyre::eyre::WrapErr;
use cs2kz::Tier;

use super::{Record, calculate_nub, calculate_pro};

#[test]
fn nub() -> color_eyre::Result<()>
{
	let mut nub_inputs =
		serde_json::from_str::<Vec<Record>>(include_str!("test-data/nub-inputs.json"))
			.context("parse nub inputs")?;
	nub_inputs.sort_unstable();
	let nub_outputs = serde_json::from_str::<Vec<u16>>(include_str!("test-data/nub-outputs.json"))
		.context("parse nub outputs")?;

	let results = calculate_nub(Tier::VeryEasy, &nub_inputs).context("calculate nub points")?;

	assert_eq!(results.points, nub_outputs);

	Ok(())
}

#[test]
fn pro() -> color_eyre::Result<()>
{
	let mut nub_inputs =
		serde_json::from_str::<Vec<Record>>(include_str!("test-data/nub-inputs.json"))
			.context("parse nub inputs")?;
	nub_inputs.sort_unstable();
	let mut pro_inputs =
		serde_json::from_str::<Vec<Record>>(include_str!("test-data/pro-inputs.json"))
			.context("parse pro inputs")?;
	pro_inputs.sort_unstable();
	let pro_outputs = serde_json::from_str::<Vec<u16>>(include_str!("test-data/pro-outputs.json"))
		.context("parse pro outputs")?;

	let mut nub_outputs =
		calculate_nub(Tier::VeryEasy, &nub_inputs).context("calculate nub points")?;
	let results = calculate_pro(Tier::VeryEasy, &pro_inputs, &nub_inputs, &mut nub_outputs)
		.context("calculate pro points")?;

	assert_eq!(results.points, pro_outputs);

	Ok(())
}
