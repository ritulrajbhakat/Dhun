use anyhow::Result;
use qrcode::render::svg;
use qrcode::QrCode;

pub fn generate_party_qr_svg(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes())?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(280, 280)
        .dark_color(svg::Color("#f43f5e")) // DHUN accent crimson
        .light_color(svg::Color("#12141a")) // Dark surface background
        .build();
    Ok(image)
}
