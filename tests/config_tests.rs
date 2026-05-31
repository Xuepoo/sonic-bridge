use sonic_bridge::config::{get_xdg_config_path, SonicConfig};

#[test]
fn test_xdg_path_resolver_and_default_loading() {
    let xdg_path = get_xdg_config_path();
    assert!(xdg_path.to_str().unwrap().contains("sonic-bridge"));

    // Test parsing default configuration values
    let default_config = SonicConfig::default();
    assert_eq!(default_config.step_size, 5.0f32);
    assert!(!default_config.onset_mode);
    assert!(!default_config.beat_mode);
    assert_eq!(default_config.onset_threshold, 0.5f32);
}

#[test]
fn test_partial_toml_parsing() {
    let toml_str = r#"
        step_size = 3.0
        onset_mode = true
        beat_mode = true
    "#;
    let parsed: SonicConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(parsed.step_size, 3.0f32);
    assert!(parsed.onset_mode);
    assert!(parsed.beat_mode);
    assert_eq!(parsed.onset_threshold, 0.5f32); // Fallback to Default
    assert!(!parsed.cache_dir.is_empty()); // Fallback to Default
}
