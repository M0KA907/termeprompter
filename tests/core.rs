use std::time::{Duration, Instant};
use std::{path::Path, process::Command};
use termeprompter::importer;
use termeprompter::mirror::mirror_row;
use termeprompter::parser::parse;
use termeprompter::render::LayoutKind;
use termeprompter::scroll::{ScrollState, Wpm, WrapLayout, MIN_ROW_WEIGHT};
use termeprompter::timing::{estimate, MockClock};

#[test]
fn wrap_layout_is_strictly_monotonic() {
    let doc = parse("one two three\n\n# Heading\n[cue:a]\n---");
    let layout = WrapLayout::build(&doc, 8);

    assert_eq!(layout.cum_words.len(), layout.rows.len() + 1);
    for pair in layout.cum_words.windows(2) {
        assert!(pair[1] > pair[0]);
        assert!(pair[1] - pair[0] >= MIN_ROW_WEIGHT);
    }
}

#[test]
fn scroll_and_eta_share_word_axis() {
    let doc = parse("one two three four five six seven eight nine ten");
    let layout = WrapLayout::build(&doc, 5);
    let start = Instant::now();
    let mut clock = MockClock { t: start };
    let mut scroll = ScrollState::new();

    scroll.set_wpm(Wpm::new(60), &clock, &layout, 1);
    scroll.resume(&clock);
    clock.t += Duration::from_secs(3);
    scroll.tick(&clock, &layout, 1);

    assert!((scroll.word_pos() - 3.0).abs() < 1e-6);
    let est = estimate(scroll.word_pos(), &layout, scroll.wpm(), Duration::ZERO);
    assert!((est.remaining.as_secs_f64() - (layout.total_words - 3.0)).abs() < 1e-6);
}

#[test]
fn wpm_clamps_at_thousand() {
    assert_eq!(Wpm::new(1000).get(), 1000);
    assert_eq!(Wpm::new(1001).get(), 1000);
}

#[test]
fn fitted_document_still_advances_on_word_axis() {
    let doc = parse("one two three");
    let layout = WrapLayout::build(&doc, 80);
    let start = Instant::now();
    let mut clock = MockClock { t: start };
    let mut scroll = ScrollState::new();

    scroll.set_wpm(Wpm::new(60), &clock, &layout, 24);
    assert!(!scroll.at_end(&layout, 24));
    scroll.resume(&clock);
    clock.t += Duration::from_secs(2);
    scroll.tick(&clock, &layout, 24);

    assert!((scroll.word_pos() - 2.0).abs() < 1e-6);
    assert_eq!(scroll.scroll_rows(&layout, 24), 0.0);
}

#[test]
fn mirror_reverses_graphemes_and_right_aligns() {
    assert_eq!(mirror_row("a e\u{301}", 5), "  e\u{301} a");
}

#[test]
fn layout_cycle_includes_horizontal_mode() {
    assert_eq!(LayoutKind::default(), LayoutKind::Horizontal);
    assert_eq!(LayoutKind::Horizontal.cycle(), LayoutKind::Trainer);
    assert_eq!(LayoutKind::Trainer.cycle(), LayoutKind::Prompt);
    assert_eq!(LayoutKind::Rehearsal.cycle(), LayoutKind::Minimal);
    assert_eq!(
        "horizontal".parse::<LayoutKind>().unwrap(),
        LayoutKind::Horizontal
    );
    assert_eq!(
        "ticker".parse::<LayoutKind>().unwrap(),
        LayoutKind::Horizontal
    );
    assert_eq!(
        "trainer".parse::<LayoutKind>().unwrap(),
        LayoutKind::Trainer
    );
    assert_eq!(
        "speedread".parse::<LayoutKind>().unwrap(),
        LayoutKind::Trainer
    );
}

#[test]
fn scroll_reaches_final_visual_row() {
    let doc = parse("one\ntwo\nthree\nfour\nfive");
    let layout = WrapLayout::build(&doc, 80);
    let start = Instant::now();
    let mut clock = MockClock { t: start };
    let mut scroll = ScrollState::new();

    scroll.set_wpm(Wpm::new(60), &clock, &layout, 3);
    scroll.resume(&clock);
    clock.t += Duration::from_secs(10);
    scroll.tick(&clock, &layout, 3);

    assert!(scroll.at_end(&layout, 3));
    assert_eq!(scroll.scroll_rows(&layout, 3), 4.0);
}

#[test]
fn folder_import_combines_text_files() {
    let root = std::env::temp_dir().join(format!("termeprompter-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("a.txt"), "alpha words").unwrap();
    std::fs::write(root.join("nested").join("b.md"), "beta words").unwrap();
    std::fs::write(root.join("slides.pptx"), "not parsed").unwrap();

    let doc = importer::load_import(&root).unwrap();
    let text = doc
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("# a.txt"));
    assert!(text.contains("alpha words"));
    assert!(text.contains("# nested/b.md"));
    assert!(text.contains("beta words"));

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn pptx_import_extracts_slide_and_notes_text() {
    let root = std::env::temp_dir().join(format!("termeprompter-pptx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let deck = root.join("talk.pptx");
    write_minimal_pptx(&root, &deck);

    let doc = importer::load_import(&deck).unwrap();
    let text = doc
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("# Slide 1"));
    assert!(text.contains("Launch & learn"));
    assert!(text.contains("## Speaker notes"));
    assert!(text.contains("Pause < breathe"));

    std::fs::remove_dir_all(&root).unwrap();
}

fn write_minimal_pptx(root: &Path, deck: &Path) {
    let build = root.join("build");
    std::fs::create_dir_all(build.join("ppt/slides")).unwrap();
    std::fs::create_dir_all(build.join("ppt/notesSlides")).unwrap();
    std::fs::write(
        build.join("ppt/slides/slide1.xml"),
        r#"<p:sld><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Launch &amp; learn</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
    )
    .unwrap();
    std::fs::write(
        build.join("ppt/notesSlides/notesSlide1.xml"),
        r#"<p:notes><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Pause &lt; breathe</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
    )
    .unwrap();

    let status = Command::new("zip")
        .arg("-q")
        .arg("-r")
        .arg(deck)
        .arg("ppt")
        .current_dir(&build)
        .status()
        .unwrap();
    assert!(status.success());
}
