use governor::clock::{Clock, DefaultClock};
use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

pub fn auth_rate_limit_layer(
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware<<DefaultClock as Clock>::Instant>> {
    let config = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(5)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}

pub fn sensitive_rate_limit_layer(
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware<<DefaultClock as Clock>::Instant>> {
    let config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(3)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}

pub fn api_rate_limit_layer(
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware<<DefaultClock as Clock>::Instant>> {
    let config = GovernorConfigBuilder::default()
        .per_second(20)
        .burst_size(60)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}
