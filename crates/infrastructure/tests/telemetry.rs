use infrastructure::telemetry;

#[test]
fn init_subscriber_does_not_panic() {
    telemetry::init_subscriber("debug", false);
}
