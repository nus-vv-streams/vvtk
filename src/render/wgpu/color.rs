use color_space::Rgb;
use regex::bytes::Regex;

pub fn parse_color(color_str: &str) -> Result<Rgb, &str> {
    if color_str.starts_with("rgb") {
        let pattern = Regex::new(r"^rgb\((\d{1,3}),(\d{1,3}),(\d{1,3})\)$").unwrap();
        // check if color_str match this regex pattern
        if !pattern.is_match(color_str.as_bytes()) {
            return Err(
                "Invalid background rgb color format, expected rgb(r,g,b) such as rgb(122,31,212)",
            );
        }

        let rgb = color_str[4..color_str.len() - 1]
            .split(',')
            .map(|s| s.parse::<u8>().unwrap())
            .collect::<Vec<_>>();
        return Ok(Rgb::new(rgb[0] as f64, rgb[1] as f64, rgb[2] as f64));
    } else if color_str.starts_with("#") {
        let hex_num = u32::from_str_radix(&color_str[1..], 16);
        if color_str.len() != 7 || hex_num.is_err() {
            return Err("Invalid background hex color format, expected #rrggbb such as #7a1fd4");
        }
        return Ok(Rgb::from_hex(hex_num.unwrap()));
    } else {
        return Err("Invalid background color format, expected rgb(r,g,b) or #rrggbb such as rgb(122,31,212) or #7a1fd4");
    }
}

pub fn parse_wgpu_color(color_str: &str) -> Result<wgpu::Color, &str> {
    parse_color(color_str).map(|rgb| wgpu::Color {
        r: rgb.r / 255.0,
        g: rgb.g / 255.0,
        b: rgb.b / 255.0,
        a: 1.0,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_bg_color() {
        assert_eq!(
            parse_color("rgb(255,122,11)").unwrap(),
            Rgb::new(255f64, 122f64, 11f64)
        );
        assert_eq!(
            parse_color("#9ef244").unwrap(),
            Rgb::new(158f64, 242f64, 68f64)
        );
        assert_eq!(
            parse_color("#9EF24A").unwrap(),
            Rgb::new(158f64, 242f64, 74f64)
        );

        assert!(parse_color("rgb(255,122,11, 0.5)").is_err());
        assert!(parse_color("rgb(255,122)").is_err());
        assert!(parse_color("rgb(255,122,11, 0.5)").is_err());
        assert!(parse_color("(255,122,11, 0.5)").is_err());

        assert!(parse_color("#9ef24").is_err());
        assert!(parse_color("#9ef2444").is_err());
        assert!(parse_color("9ef244").is_err());
        assert!(parse_color("#9IJ444").is_err());
    }
}
