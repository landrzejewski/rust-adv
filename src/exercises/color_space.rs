use std::fmt;
use std::ops::Deref;

// ============================
// Error type
// ============================

#[derive(Debug, PartialEq)]
enum ColorError {
    InvalidHue(f64),
    InvalidSaturation(f64),
    InvalidLightness(f64),
    InvalidHexFormat(String),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidHue(h) => write!(f, "Invalid hue: {h} (must be 0.0..360.0)"),
            ColorError::InvalidSaturation(s) => {
                write!(f, "Invalid saturation: {s} (must be 0.0..=1.0)")
            }
            ColorError::InvalidLightness(l) => {
                write!(f, "Invalid lightness: {l} (must be 0.0..=1.0)")
            }
            ColorError::InvalidHexFormat(s) => {
                write!(f, "Invalid hex color: '{s}' (expected #RRGGBB)")
            }
        }
    }
}

impl std::error::Error for ColorError {}

// ============================
// Rgb — always valid by construction
// ============================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb(r, g, b)
    }

    fn brightness(&self) -> f64 {
        // Perceived brightness (ITU-R BT.601)
        (0.299 * self.0 as f64 + 0.587 * self.1 as f64 + 0.114 * self.2 as f64) / 255.0
    }

    fn to_grayscale(&self) -> Rgb {
        let gray = (0.299 * self.0 as f64 + 0.587 * self.1 as f64 + 0.114 * self.2 as f64) as u8;
        Rgb(gray, gray, gray)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({}, {}, {})", self.0, self.1, self.2)
    }
}

// ============================
// HexColor — validated via TryFrom<&str>
// ============================

#[derive(Debug, Clone, PartialEq, Eq)]
struct HexColor(String);

impl TryFrom<&str> for HexColor {
    type Error = ColorError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() != 7 || !s.starts_with('#') {
            return Err(ColorError::InvalidHexFormat(s.to_string()));
        }
        // Validate all chars after # are hex digits
        if !s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ColorError::InvalidHexFormat(s.to_string()));
        }
        // Normalize to uppercase
        Ok(HexColor(format!("#{}", s[1..].to_ascii_uppercase())))
    }
}

impl Deref for HexColor {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================
// Hsl — validated via TryFrom<(f64,f64,f64)>
// ============================

#[derive(Debug, Clone, Copy, PartialEq)]
struct Hsl(f64, f64, f64); // h, s, l

impl TryFrom<(f64, f64, f64)> for Hsl {
    type Error = ColorError;

    fn try_from((h, s, l): (f64, f64, f64)) -> Result<Self, Self::Error> {
        if !(0.0..360.0).contains(&h) {
            return Err(ColorError::InvalidHue(h));
        }
        if !(0.0..=1.0).contains(&s) {
            return Err(ColorError::InvalidSaturation(s));
        }
        if !(0.0..=1.0).contains(&l) {
            return Err(ColorError::InvalidLightness(l));
        }
        Ok(Hsl(h, s, l))
    }
}

impl fmt::Display for Hsl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hsl({:.0}, {:.0}%, {:.0}%)", self.0, self.1 * 100.0, self.2 * 100.0)
    }
}

// ============================
// Conversions: triangle of From / TryFrom
// ============================

// Rgb -> HexColor (infallible)
impl From<Rgb> for HexColor {
    fn from(rgb: Rgb) -> Self {
        HexColor(format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2))
    }
}

// HexColor -> Rgb (infallible — HexColor is already validated)
impl From<&HexColor> for Rgb {
    fn from(hex: &HexColor) -> Self {
        let r = u8::from_str_radix(&hex.0[1..3], 16).unwrap();
        let g = u8::from_str_radix(&hex.0[3..5], 16).unwrap();
        let b = u8::from_str_radix(&hex.0[5..7], 16).unwrap();
        Rgb(r, g, b)
    }
}

// Rgb -> Hsl (infallible — any RGB maps to valid HSL)
impl From<Rgb> for Hsl {
    fn from(rgb: Rgb) -> Self {
        let r = rgb.0 as f64 / 255.0;
        let g = rgb.1 as f64 / 255.0;
        let b = rgb.2 as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f64::EPSILON {
            // Achromatic
            return Hsl(0.0, 0.0, l);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - r).abs() < f64::EPSILON {
            let mut h = (g - b) / d;
            if g < b {
                h += 6.0;
            }
            h
        } else if (max - g).abs() < f64::EPSILON {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        Hsl(h * 60.0, s, l)
    }
}

// ============================
// Sealed trait: ColorSpace
// ============================

mod sealed {
    pub trait Sealed {}
}

trait ColorSpace: sealed::Sealed + fmt::Display {
    fn to_rgb(&self) -> Rgb;
    fn label(&self) -> &'static str;
}

impl sealed::Sealed for Rgb {}
impl sealed::Sealed for HexColor {}
impl sealed::Sealed for Hsl {}

impl ColorSpace for Rgb {
    fn to_rgb(&self) -> Rgb {
        *self
    }
    fn label(&self) -> &'static str {
        "RGB"
    }
}

impl ColorSpace for HexColor {
    fn to_rgb(&self) -> Rgb {
        Rgb::from(self)
    }
    fn label(&self) -> &'static str {
        "Hex"
    }
}

impl ColorSpace for Hsl {
    fn to_rgb(&self) -> Rgb {
        // HSL to RGB conversion
        let Hsl(h, s, l) = *self;

        if s.abs() < f64::EPSILON {
            let v = (l * 255.0) as u8;
            return Rgb(v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let hue_to_rgb = |t: f64| -> f64 {
            let mut t = t;
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        let h_norm = h / 360.0;
        Rgb(
            (hue_to_rgb(h_norm + 1.0 / 3.0) * 255.0) as u8,
            (hue_to_rgb(h_norm) * 255.0) as u8,
            (hue_to_rgb(h_norm - 1.0 / 3.0) * 255.0) as u8,
        )
    }
    fn label(&self) -> &'static str {
        "HSL"
    }
}

// ============================
// Extension trait: ColorPaletteExt on [Rgb]
// ============================

trait ColorPaletteExt {
    fn average_brightness(&self) -> f64;
    fn most_saturated(&self) -> Option<&Rgb>;
    fn to_grayscale(&self) -> Vec<Rgb>;
}

impl ColorPaletteExt for [Rgb] {
    fn average_brightness(&self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        let total: f64 = self.iter().map(|c| c.brightness()).sum();
        total / self.len() as f64
    }

    fn most_saturated(&self) -> Option<&Rgb> {
        self.iter().max_by(|a, b| {
            let sa = Hsl::from(**a).1;
            let sb = Hsl::from(**b).1;
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn to_grayscale(&self) -> Vec<Rgb> {
        self.iter().map(|c| c.to_grayscale()).collect()
    }
}

// ============================
// Helper: display via dyn ColorSpace
// ============================

fn describe_color(color: &dyn ColorSpace) {
    let rgb = color.to_rgb();
    println!("  [{}] {} => {}", color.label(), color, rgb);
}

// ============================
// Demonstration
// ============================

pub fn run() {
    println!("=== Exercise 11: Color Space Library ===\n");

    // --- Rgb (always valid) ---
    println!("--- RGB colors ---");
    let red = Rgb::new(255, 0, 0);
    let green = Rgb::new(0, 128, 0);
    let sky = Rgb::new(135, 206, 235);
    println!("  {red} (brightness: {:.3})", red.brightness());
    println!("  {green} (brightness: {:.3})", green.brightness());
    println!("  {sky} (brightness: {:.3})", sky.brightness());

    // --- HexColor (validated) ---
    println!("\n--- HexColor validation ---");
    let hex_ok = HexColor::try_from("#ff8800");
    let hex_bad1 = HexColor::try_from("ff8800");
    let hex_bad2 = HexColor::try_from("#ZZZZZZ");
    let hex_bad3 = HexColor::try_from("#fff");
    println!("  \"#ff8800\" => {:?}", hex_ok);
    println!("  \"ff8800\"  => {:?}", hex_bad1);
    println!("  \"#ZZZZZZ\" => {:?}", hex_bad2);
    println!("  \"#fff\"    => {:?}", hex_bad3);

    // Deref: use &str methods on HexColor
    let hex = hex_ok.unwrap();
    println!("  HexColor len via Deref: {}", hex.len());
    println!("  Starts with #: {}", hex.starts_with('#'));

    // --- Hsl (validated) ---
    println!("\n--- HSL validation ---");
    let hsl_ok = Hsl::try_from((200.0, 0.8, 0.5));
    let hsl_bad_h = Hsl::try_from((400.0, 0.5, 0.5));
    let hsl_bad_s = Hsl::try_from((100.0, 1.5, 0.5));
    let hsl_bad_l = Hsl::try_from((100.0, 0.5, -0.1));
    println!("  (200, 0.8, 0.5) => {:?}", hsl_ok);
    println!("  (400, 0.5, 0.5) => {:?}", hsl_bad_h);
    println!("  (100, 1.5, 0.5) => {:?}", hsl_bad_s);
    println!("  (100, 0.5, -0.1) => {:?}", hsl_bad_l);

    // --- Conversions ---
    println!("\n--- Conversions ---");
    let orange = Rgb::new(255, 136, 0);

    // Rgb -> HexColor
    let orange_hex = HexColor::from(orange);
    println!("  {orange} => {orange_hex}");

    // HexColor -> Rgb
    let back = Rgb::from(&orange_hex);
    println!("  {orange_hex} => {back}");
    assert_eq!(orange, back);
    println!("  Round-trip: OK (Rgb -> Hex -> Rgb identical)");

    // Rgb -> Hsl
    let orange_hsl = Hsl::from(orange);
    println!("  {orange} => {orange_hsl}");

    // Hsl -> Rgb (via ColorSpace trait)
    let orange_back = orange_hsl.to_rgb();
    println!("  {orange_hsl} => {orange_back}");

    // --- Sealed ColorSpace trait via dyn dispatch ---
    println!("\n--- ColorSpace (sealed trait, dyn dispatch) ---");
    let colors: Vec<Box<dyn ColorSpace>> = vec![
        Box::new(Rgb::new(255, 0, 0)),
        Box::new(HexColor::try_from("#00ff00").unwrap()),
        Box::new(Hsl::try_from((240.0, 1.0, 0.5)).unwrap()),
    ];
    for c in &colors {
        describe_color(c.as_ref());
    }

    // --- Extension trait on palette ---
    println!("\n--- ColorPaletteExt ---");
    let palette = [
        Rgb::new(255, 0, 0),   // pure red
        Rgb::new(0, 255, 0),   // pure green
        Rgb::new(0, 0, 255),   // pure blue
        Rgb::new(128, 128, 128), // gray
        Rgb::new(255, 200, 0), // yellow-orange
    ];
    println!("  Average brightness: {:.3}", palette.average_brightness());
    if let Some(most_sat) = palette.most_saturated() {
        println!("  Most saturated: {most_sat}");
    }
    println!("  Grayscale:");
    for (orig, gray) in palette.iter().zip(palette.to_grayscale()) {
        println!("    {orig} => {gray}");
    }

    // --- Custom error enum ---
    println!("\n--- Error enum Display ---");
    let errors = [
        ColorError::InvalidHue(400.0),
        ColorError::InvalidSaturation(1.5),
        ColorError::InvalidLightness(-0.2),
        ColorError::InvalidHexFormat("bad".to_string()),
    ];
    for e in &errors {
        println!("  {e}");
    }
}
