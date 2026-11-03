use gpui::{
    Font, FontFallbacks, FontRun, FontWeight, PlatformTextSystem, RenderGlyphParams, point, px,
};
use gpui_wgpu::CosmicTextSystem;
use std::borrow::Cow;

const VARIABLE: &[u8] = include_bytes!("fonts/gpui-var-test.ttf");
const STATIC: &[u8] = include_bytes!("fonts/gpui-static-test.ttf");

fn weighted(family: &str, weight: FontWeight) -> Font {
    let mut font = gpui::font(family.to_owned());
    font.weight = weight;
    font
}

fn ink(fallback: bool, weight: FontWeight) -> u64 {
    let primary = if fallback {
        "GpuiStaticTest"
    } else {
        "GpuiVarTest"
    };
    let system = CosmicTextSystem::new(primary);
    system
        .add_fonts(vec![Cow::Borrowed(VARIABLE), Cow::Borrowed(STATIC)])
        .unwrap();
    let mut font = weighted(primary, weight);
    if fallback {
        font.fallbacks = Some(FontFallbacks::from_fonts(vec!["GpuiVarTest".into()]));
    }
    let primary_id = system.font_id(&font).unwrap();
    let layout = system.layout_line(
        "中",
        px(32.),
        &[FontRun {
            len: "中".len(),
            font_id: primary_id,
        }],
    );
    let run = layout
        .runs
        .first()
        .expect("Chinese glyph must produce a run");
    if fallback {
        assert_ne!(
            run.font_id, primary_id,
            "Chinese glyph must use the variable fallback"
        );
    }
    let params = RenderGlyphParams {
        font_id: run.font_id,
        glyph_id: run.glyphs.first().unwrap().id,
        font_size: px(32.),
        subpixel_variant: point(0, 0),
        scale_factor: 1.,
        is_emoji: false,
        subpixel_rendering: false,
        dilation: 0,
    };
    let bounds = system.glyph_raster_bounds(&params).unwrap();
    let (_, bytes) = system.rasterize_glyph(&params, bounds).unwrap();
    bytes.iter().map(|&pixel| pixel as u64).sum()
}

#[test]
fn variable_primary_rasterizes_requested_weights() {
    assert!(ink(false, FontWeight::BOLD) > ink(false, FontWeight::NORMAL));
}

#[test]
fn variable_fallback_rasterizes_requested_weights() {
    assert!(ink(true, FontWeight::BOLD) > ink(true, FontWeight::NORMAL));
}
