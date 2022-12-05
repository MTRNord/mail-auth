/*
 * Copyright (c) 2020-2022, Stalwart Labs Ltd.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use mail_auth::{AuthenticatedMessage, DkimResult, Resolver};

const TEST_MESSAGE: &str = include_str!("../resources/arc/001.txt");

#[tokio::main]
async fn main() {
    // Create a resolver using Cloudflare DNS
    let resolver = Resolver::new_cloudflare_tls().unwrap();

    // Parse message
    let authenticated_message = AuthenticatedMessage::parse(TEST_MESSAGE.as_bytes()).unwrap();

    // Validate ARC chain
    let result = resolver.verify_arc(&authenticated_message).await;

    // Make sure ARC passed verification
    assert_eq!(result.result(), &DkimResult::Pass);
}
