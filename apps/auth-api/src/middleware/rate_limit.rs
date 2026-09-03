use governor::clock::{Clock, DefaultClock};
use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

pub fn rate_limit_layer(
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware<<DefaultClock as Clock>::Instant>> {
    let config = GovernorConfigBuilder::default()
        .per_second(5)
        .burst_size(20)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}
