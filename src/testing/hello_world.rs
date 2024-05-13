use cs2kz_api_macros::integration_test;

use crate::testing::{self, TestResult};

#[integration_test]
async fn hello_world(ctx: &Context) -> TestResult {
	let response = ctx.http_client().get(ctx.url("/")).send().await?;

	testing::assert_eq!(response.status(), 200);

	let response_body = response.text().await?;

	testing::assert_eq!(response_body, "(͡ ͡° ͜ つ ͡͡°)");

	Ok(())
}
