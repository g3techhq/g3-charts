//! Value labels for axes, tooltips and data tables.

/// How a value is written in labels, tooltips and the data table.
#[allow(
    unpredictable_function_pointer_comparisons,
    reason = "props compare formats to skip re-renders; a false miss only re-renders"
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueFormat {
    /// A plain number with thousands separators: `12,345.6`.
    Number {
        /// Digits after the decimal point.
        decimals: u8,
    },
    /// A fraction as a percentage: `0.425` is `42.5%`.
    Percent {
        /// Digits after the decimal point.
        decimals: u8,
    },
    /// Money with a leading symbol: `$12,345.00`, `-$3.50`.
    Currency {
        /// The symbol, such as `"$"` or `"€"`.
        symbol: &'static str,
        /// Digits after the decimal point.
        decimals: u8,
    },
    /// A short magnitude: `950`, `12.3k`, `4.5M`, `1.2B`, `3T`.
    Compact {
        /// An optional leading symbol, such as `"$"`.
        symbol: &'static str,
    },
    /// Your own formatter.
    Custom(fn(f64) -> String),
}

impl Default for ValueFormat {
    fn default() -> Self {
        ValueFormat::Number { decimals: 0 }
    }
}

impl ValueFormat {
    /// The label for a value. A value that is not finite is `–`.
    ///
    /// ```
    /// use g3_charts::geom::ValueFormat;
    ///
    /// assert_eq!(ValueFormat::Number { decimals: 1 }.format(12345.67), "12,345.7");
    /// assert_eq!(ValueFormat::Percent { decimals: 1 }.format(0.425), "42.5%");
    /// assert_eq!(ValueFormat::Currency { symbol: "$", decimals: 2 }.format(-3.5), "-$3.50");
    /// assert_eq!(ValueFormat::Compact { symbol: "$" }.format(1_250_000.0), "$1.25M");
    /// ```
    pub fn format(&self, value: f64) -> String {
        if !value.is_finite() {
            return "–".into();
        }
        match *self {
            ValueFormat::Number { decimals } => grouped(value, decimals),
            ValueFormat::Percent { decimals } => format!("{}%", grouped(value * 100.0, decimals)),
            ValueFormat::Currency { symbol, decimals } => {
                let sign = if is_negative(value, decimals) {
                    "-"
                } else {
                    ""
                };
                format!("{sign}{symbol}{}", grouped(value.abs(), decimals))
            }
            ValueFormat::Compact { symbol } => compact(value, symbol),
            ValueFormat::Custom(format) => format(value),
        }
    }

    /// [`format`](Self::format) with a `+` on positive values, for changes
    /// such as drift from a target.
    pub fn format_signed(&self, value: f64) -> String {
        let text = self.format(value);
        if value > 0.0
            && !text.starts_with('-')
            && text.chars().any(|c| c.is_ascii_digit() && c != '0')
        {
            format!("+{text}")
        } else {
            text
        }
    }

    /// The format an axis uses at a tick step: a percent or number keeps
    /// only the decimals the step needs.
    pub fn for_step(&self, step: f64) -> ValueFormat {
        let need = |scale: f64| super::step_decimals(step * scale).min(6) as u8;
        match *self {
            ValueFormat::Number { .. } => ValueFormat::Number {
                decimals: need(1.0),
            },
            ValueFormat::Percent { .. } => ValueFormat::Percent {
                decimals: need(100.0),
            },
            ValueFormat::Currency { symbol, decimals } => ValueFormat::Currency {
                symbol,
                decimals: if step >= 1.0 { 0 } else { decimals },
            },
            other => other,
        }
    }
}

fn is_negative(value: f64, decimals: u8) -> bool {
    let factor = 10f64.powi(i32::from(decimals));
    (value * factor).round() < 0.0
}

/// Fixed decimals with comma thousands separators, never `-0`.
fn grouped(value: f64, decimals: u8) -> String {
    let negative = is_negative(value, decimals);
    let text = format!("{:.*}", usize::from(decimals), value.abs());
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text.as_str(), None), |(w, f)| (w, Some(f)));
    let mut out = String::with_capacity(text.len() + whole.len() / 3 + 1);
    if negative {
        out.push('-');
    }
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    if let Some(fraction) = fraction {
        out.push('.');
        out.push_str(fraction);
    }
    out
}

fn compact(value: f64, symbol: &str) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let magnitude = value.abs();
    let (scaled, suffix) = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "k")]
        .into_iter()
        .find(|(unit, _)| magnitude >= *unit * 0.9995)
        .map_or((magnitude, ""), |(unit, suffix)| (magnitude / unit, suffix));
    // Three significant digits: 1.25M, 12.5M, 125M.
    let decimals = if scaled >= 100.0 || suffix.is_empty() && scaled >= 10.0 {
        0
    } else if scaled >= 10.0 {
        1
    } else {
        2
    };
    let mut text = format!("{scaled:.decimals$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "0" {
        return format!("{symbol}0");
    }
    format!("{sign}{symbol}{text}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_group_thousands() {
        let f = ValueFormat::Number { decimals: 0 };
        assert_eq!(f.format(0.0), "0");
        assert_eq!(f.format(999.0), "999");
        assert_eq!(f.format(1_000.0), "1,000");
        assert_eq!(f.format(-1_234_567.0), "-1,234,567");
        assert_eq!(f.format(-0.2), "0");
        assert_eq!(f.format(f64::NAN), "–");
        assert_eq!(ValueFormat::Number { decimals: 2 }.format(0.005), "0.01");
    }

    #[test]
    fn currency_puts_the_sign_before_the_symbol() {
        let usd = ValueFormat::Currency {
            symbol: "$",
            decimals: 2,
        };
        assert_eq!(usd.format(1_234.5), "$1,234.50");
        assert_eq!(usd.format(-0.001), "$0.00");
        assert_eq!(usd.format_signed(12.0), "+$12.00");
        assert_eq!(usd.format_signed(-12.0), "-$12.00");
        assert_eq!(usd.format_signed(0.0), "$0.00");
    }

    #[test]
    fn compact_keeps_three_significant_digits() {
        let f = ValueFormat::Compact { symbol: "" };
        assert_eq!(f.format(950.0), "950");
        assert_eq!(f.format(12.34), "12");
        assert_eq!(f.format(1.5), "1.5");
        assert_eq!(f.format(1_000.0), "1k");
        assert_eq!(f.format(12_345.0), "12.3k");
        assert_eq!(f.format(999_999.0), "1M");
        assert_eq!(f.format(-4_500_000.0), "-4.5M");
        assert_eq!(f.format(2.1e12), "2.1T");
        assert_eq!(f.format(0.0), "0");
    }

    #[test]
    fn percent_and_axis_steps() {
        let p = ValueFormat::Percent { decimals: 1 };
        assert_eq!(p.format(1.0), "100.0%");
        assert_eq!(p.format_signed(0.012), "+1.2%");
        assert_eq!(p.for_step(0.05), ValueFormat::Percent { decimals: 0 });
        assert_eq!(p.for_step(0.005), ValueFormat::Percent { decimals: 1 });
        let n = ValueFormat::Number { decimals: 3 };
        assert_eq!(n.for_step(0.25), ValueFormat::Number { decimals: 2 });
        assert_eq!(n.for_step(500.0), ValueFormat::Number { decimals: 0 });
    }
}
