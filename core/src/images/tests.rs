use super::*;

const SVG_HEAD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- a comment -->
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 220 140"><rect width="10" height="10"/></svg>"#;

#[test]
fn svg_is_recognised_by_its_root() {
    assert!(is_svg(SVG_HEAD.as_bytes()));
    assert!(is_svg(b"\xEF\xBB\xBF<svg>"));
    assert!(!is_svg(b"<html><body><svg viewBox=\"0 0 1 1\"/></body></html>"), "SVG inside a page is not an image");
    assert!(!is_svg(b"<svgx/>"));
    assert!(!is_svg(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn svg_size_from_its_root() {
    let size = |root: &str| super::size(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" {root}></svg>").as_bytes());
    assert_eq!(size(r#"viewBox="0 0 220 140""#), Some((220.0, 140.0)), "the viewBox");
    assert_eq!(size(r#"width="300" height="200px" viewBox="0 0 30 20""#), Some((300.0, 200.0)), "px over the viewBox");
    assert_eq!(size(r#"width="3in" height="72pt""#), Some((288.0, 96.0)), "absolute units");
    assert_eq!(size(r#"width="100%" height="100%" viewBox="0 0 741 1089""#), Some((741.0, 1089.0)), "percentages: the viewBox");
    assert_eq!(size(r#"width="100" viewBox="0,0,200,50""#), Some((100.0, 25.0)), "one side: the other by the viewBox");
    assert_eq!(size(r#"stroke-width="3" viewBox="0 0 10 20""#), Some((10.0, 20.0)), "not an attribute ending in width");
    assert_eq!(size(r#"width="100%""#), None, "nothing absolute");
    assert_eq!(super::size(b"<svg"), None, "broken");
}

#[test]
fn raster_fits_the_box_up_to_four_times_its_size() {
    let svg = SVG_HEAD.as_bytes();
    let r = raster(svg, 1100, 1100).unwrap();
    assert_eq!((r.width, r.height), (880, 560), "at most four times 220 × 140");
    let r = raster(svg, 440, 1000).unwrap();
    assert_eq!((r.width, r.height), (440, 280), "fits the box");
    assert_eq!(r.pixels.len(), 440 * 280 * 4);
    assert!(Arc::ptr_eq(&r, &raster(svg, 440, 1000).unwrap()), "kept");
    assert!(raster(b"<svg viewBox=\"0 0 10 10\"><path d=", 100, 100).is_none(), "broken");
    assert!(raster(b"\x89PNG\r\n\x1a\n", 100, 100).is_none(), "not SVG");
}

#[test]
fn line_art_is_told_from_pictures() {
    let drawing = |fill: &str| {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path d="M10 10 L90 90 M90 10 L10 90" stroke="{fill}" stroke-width="8"/></svg>"#
        );
        raster(svg.as_bytes(), 300, 300).unwrap().ink
    };
    assert!(drawing("#000"), "black lines on transparency");
    assert!(drawing("#333"), "dark grey lines");
    assert!(!drawing("#c33"), "red lines");
    let opaque = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#fff"/><path d="M0 0 L10 10" stroke="#000"/></svg>"##;
    assert!(!raster(opaque.as_bytes(), 100, 100).unwrap().ink, "an engraving on white paper");
}

#[test]
fn ink_test_on_pixels() {
    let (w, h) = (100u32, 100u32);
    let image = |pixel: &dyn Fn(u32, u32) -> [u8; 4]| -> Vec<u8> { (0..h).flat_map(|y| (0..w).flat_map(move |x| pixel(x, y))).collect() };
    let lines = image(&|x, _| {
        if x % 10 == 0 {
            [0, 0, 0, 255]
        } else if x % 10 == 1 {
            [40, 40, 40, 160]
        } else {
            [0; 4]
        }
    });
    assert!(is_ink(&lines, w, h, w as usize * 4), "black lines with grey antialiasing");
    let colour = image(&|x, _| if x < 10 { [0; 4] } else { [200, 120, 40, 255] });
    assert!(!is_ink(&colour, w, h, w as usize * 4), "a colour picture with transparent edges");
    let paper = image(&|x, _| if x % 10 == 0 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
    assert!(!is_ink(&paper, w, h, w as usize * 4), "black on opaque white");
    assert!(!is_ink(&image(&|_, _| [0; 4]), w, h, w as usize * 4), "nothing at all");
}
