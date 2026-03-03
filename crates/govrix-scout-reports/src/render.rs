//! SVG chart rendering helpers for HTML reports.

/// Generate an inline SVG bar chart.
///
/// `data` is a slice of `(label, value)` pairs. Returns a `<svg>…</svg>`
/// string ready to embed directly in HTML. Returns an empty string when
/// `data` is empty or every value is zero.
pub fn svg_bar_chart(data: &[(String, f64)], width: u32, height: u32) -> String {
    if data.is_empty() {
        return String::new();
    }
    let max_val = data.iter().map(|(_, v)| *v).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return String::new();
    }

    let bar_width = width as f64 / data.len() as f64;
    let chart_height = (height - 30) as f64; // leave room for labels

    let mut bars = String::new();
    for (i, (label, val)) in data.iter().enumerate() {
        let x = i as f64 * bar_width + 2.0;
        let bar_h = (val / max_val) * chart_height;
        let y = chart_height - bar_h;
        bars.push_str(&format!(
            "<rect x=\"{x:.0}\" y=\"{y:.0}\" width=\"{w:.0}\" height=\"{bar_h:.0}\" fill=\"#4ade80\" rx=\"2\"/>",
            w = bar_width - 4.0
        ));
        // label below bar
        bars.push_str(&format!(
            "<text x=\"{cx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-size=\"9\" fill=\"#aaa\">{label}</text>",
            cx = x + (bar_width - 4.0) / 2.0,
            ly = chart_height + 18.0,
            label = if label.len() > 8 { &label[..8] } else { label }
        ));
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" style="background:#1a1a1a;border-radius:8px">{bars}</svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_bar_chart_returns_svg_element() {
        let data = vec![("agent1".to_string(), 100.0), ("agent2".to_string(), 200.0)];
        let svg = svg_bar_chart(&data, 400, 200);
        assert!(svg.starts_with("<svg"), "expected SVG element, got: {svg}");
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("rect"));
    }

    #[test]
    fn svg_bar_chart_empty_returns_empty() {
        let svg = svg_bar_chart(&[], 400, 200);
        assert!(svg.is_empty(), "expected empty string for empty data");
    }

    #[test]
    fn svg_bar_chart_all_zeros_returns_empty() {
        let data = vec![("a".to_string(), 0.0), ("b".to_string(), 0.0)];
        let svg = svg_bar_chart(&data, 400, 200);
        assert!(svg.is_empty());
    }
}
