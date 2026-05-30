use sonic_bridge::renderer::AlrcDoc;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

#[test]
fn test_alrc_parsing_and_document_structure() {
    // 1. Create a dummy synthetic .alrc file
    let alrc_content = r#"[ti:Synesthetic Odyssey]
[ar:Acoustic Agent]
[al:Synesthesia Core]
[length:03:45]
[@alrc_version:v1.0]

[00:00.00] [和弦:Silent | 动态:Quiet | 音色:Muffled] [Synesthesia: A soft mist fills the dark corridor.] Warmup sequence.
[00:10.50] [和弦:C | 动态:Mezzo | 音色:Bright] [Synesthesia: A golden beam piercing through clouds.] The journey begins with pure energy.
[00:30.00] [和弦:Am | 动态:Forte | 音色:Warm] [Synesthesia: Velvet indigo drapes falling on concrete.] Intense melodic shadows scraping the field.
"#;

    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join("test_song.alrc");

    let mut file = File::create(&temp_file_path).expect("Failed to create temp test ALRC file");
    file.write_all(alrc_content.as_bytes())
        .expect("Failed to write to temp test ALRC file");

    // 2. Load and parse using AlrcDoc
    let doc = AlrcDoc::load_from_file(&temp_file_path).expect("Failed to load and parse ALRC file");

    // 3. Clean up
    let _ = std::fs::remove_file(&temp_file_path);

    // 4. Assertions on metadata
    assert_eq!(doc.title, "Synesthetic Odyssey");
    assert_eq!(doc.artist, "Acoustic Agent");
    assert_eq!(doc.total_duration, Duration::from_secs(3 * 60 + 45));

    // 5. Assertions on segments
    assert_eq!(doc.segments.len(), 3);

    // Assert segment 0
    assert_eq!(doc.segments[0].time_offset, Duration::from_secs(0));
    assert_eq!(doc.segments[0].chord, "Silent");
    assert_eq!(doc.segments[0].dynamic, "Quiet");
    assert_eq!(doc.segments[0].timbre, "Muffled");
    assert_eq!(
        doc.segments[0].synesthesia,
        "A soft mist fills the dark corridor."
    );
    assert_eq!(doc.segments[0].critique, "Warmup sequence.");

    // Assert segment 1
    assert_eq!(doc.segments[1].time_offset, Duration::from_millis(10_500));
    assert_eq!(doc.segments[1].chord, "C");
    assert_eq!(doc.segments[1].dynamic, "Mezzo");
    assert_eq!(doc.segments[1].timbre, "Bright");
    assert_eq!(
        doc.segments[1].synesthesia,
        "A golden beam piercing through clouds."
    );
    assert_eq!(
        doc.segments[1].critique,
        "The journey begins with pure energy."
    );

    // Assert segment 2
    assert_eq!(doc.segments[2].time_offset, Duration::from_secs(30));
    assert_eq!(doc.segments[2].chord, "Am");
    assert_eq!(doc.segments[2].dynamic, "Forte");
    assert_eq!(doc.segments[2].timbre, "Warm");
    assert_eq!(
        doc.segments[2].synesthesia,
        "Velvet indigo drapes falling on concrete."
    );
    assert_eq!(
        doc.segments[2].critique,
        "Intense melodic shadows scraping the field."
    );
}
