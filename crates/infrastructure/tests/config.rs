use infrastructure::config;
use secrecy::ExposeSecret;

#[test]
fn loads_configuration_from_environment() {
    std::env::set_var("APP_DATABASE__URL", "postgres://u:p@host/db");
    std::env::set_var("APP_SERVER__PORT", "9090");
    std::env::set_var("APP_AUTH__SECRET", "test-secret");

    let settings = config::configuration().expect("config loads");

    assert_eq!(
        settings.database.url.expose_secret(),
        "postgres://u:p@host/db"
    );
    assert_eq!(settings.server.port, 9090);
    assert_eq!(settings.auth.secret.expose_secret(), "test-secret");

    std::env::remove_var("APP_DATABASE__URL");
    std::env::remove_var("APP_SERVER__PORT");
    std::env::remove_var("APP_AUTH__SECRET");
}
