use sonic_bridge::config::{get_xdg_config_path, SonicConfig};

#[test]
fn test_xdg_path_resolver_and_default_loading() {
    let xdg_path = get_xdg_config_path();
    assert!(xdg_path.to_str().unwrap().contains("sonic-bridge"));

    // Test parsing default configuration values
    let default_config = SonicConfig::default();
    assert_eq!(default_config.step_size, 5.0f32);
    assert!(!default_config.onset_mode);
    assert_eq!(default_config.onset_threshold, 0.5f32);
}
