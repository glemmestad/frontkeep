//! Pure forecast math for the cost rollup: ordinary least-squares over a month's
//! cumulative spend, extrapolated to end-of-month with a confidence band. No I/O,
//! no clock — the caller passes the day index so a synthetic series is fully
//! deterministic in tests.

/// A least-squares fit of `y = slope*x + intercept`.
#[derive(Debug, Clone, Copy)]
pub struct Fit {
    pub slope: f64,
    pub intercept: f64,
    pub r2: f64,
    pub residual_std: f64,
    pub n: usize,
}

/// End-of-month projection with a band; `eom` is the fitted value at the last day
/// of the month, the band widens with the days still unspent.
#[derive(Debug, Clone, Copy)]
pub struct Forecast {
    pub eom: f64,
    pub low: f64,
    pub high: f64,
}

/// Fit a line through `(day_index, cumulative_usd)` points. Returns `None` when
/// there is too little history (<3 points) or `x` has no variance (a single day
/// repeated) — both cases where a slope would be fabricated rather than measured.
pub fn linreg(points: &[(f64, f64)]) -> Option<Fit> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    let nf = n as f64;
    let mean_x = points.iter().map(|p| p.0).sum::<f64>() / nf;
    let mean_y = points.iter().map(|p| p.1).sum::<f64>() / nf;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    for &(x, y) in points {
        let dx = x - mean_x;
        let dy = y - mean_y;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx == 0.0 {
        return None;
    }
    let slope = sxy / sxx;
    let intercept = mean_y - slope * mean_x;
    let ss_res: f64 = points
        .iter()
        .map(|&(x, y)| {
            let e = y - (slope * x + intercept);
            e * e
        })
        .sum();
    let r2 = if syy == 0.0 { 1.0 } else { 1.0 - ss_res / syy };
    // Unbiased residual standard deviation (n-2 degrees of freedom for a line).
    let residual_std = (ss_res / (nf - 2.0)).max(0.0).sqrt();
    Some(Fit {
        slope,
        intercept,
        r2,
        residual_std,
        n,
    })
}

/// Project the fitted line to `days_in_month`. The band scales with the residual
/// scatter and the number of days still to be spent, so it is widest early in the
/// month and collapses to zero on the last day.
pub fn forecast_eom(fit: &Fit, today_index: f64, days_in_month: f64) -> Forecast {
    let eom = (fit.slope * days_in_month + fit.intercept).max(0.0);
    let days_remaining = (days_in_month - today_index).max(0.0);
    let band = fit.residual_std * days_remaining.sqrt();
    Forecast {
        eom,
        low: (eom - band).max(0.0),
        high: eom + band,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_line_recovers_slope_with_tight_band() {
        // y = 2x exactly: slope 2, r2 1, no residual scatter.
        let pts: Vec<(f64, f64)> = (1..=10).map(|d| (d as f64, 2.0 * d as f64)).collect();
        let fit = linreg(&pts).unwrap();
        assert!((fit.slope - 2.0).abs() < 1e-9);
        assert!((fit.r2 - 1.0).abs() < 1e-9);
        assert!(fit.residual_std < 1e-9);
        let f = forecast_eom(&fit, 10.0, 30.0);
        assert!((f.eom - 60.0).abs() < 1e-9);
        assert!(
            (f.high - f.low).abs() < 1e-9,
            "band should be ~0 on a clean line"
        );
    }

    #[test]
    fn noisy_series_widens_the_band() {
        let clean: Vec<(f64, f64)> = (1..=10).map(|d| (d as f64, 5.0 * d as f64)).collect();
        let noisy: Vec<(f64, f64)> = (1..=10)
            .map(|d| {
                let bump = if d % 2 == 0 { 12.0 } else { -12.0 };
                (d as f64, 5.0 * d as f64 + bump)
            })
            .collect();
        let cf = forecast_eom(&linreg(&clean).unwrap(), 10.0, 30.0);
        let nf = forecast_eom(&linreg(&noisy).unwrap(), 10.0, 30.0);
        assert!(
            (nf.high - nf.low) > (cf.high - cf.low),
            "noisy band must be wider"
        );
    }

    #[test]
    fn too_little_history_is_none() {
        assert!(linreg(&[(1.0, 1.0), (2.0, 2.0)]).is_none());
        // No variance in x → slope undefined.
        assert!(linreg(&[(3.0, 1.0), (3.0, 2.0), (3.0, 3.0)]).is_none());
    }

    #[test]
    fn band_collapses_on_last_day() {
        let pts: Vec<(f64, f64)> = (1..=10)
            .map(|d| {
                (
                    d as f64,
                    3.0 * d as f64 + if d % 2 == 0 { 4.0 } else { -4.0 },
                )
            })
            .collect();
        let fit = linreg(&pts).unwrap();
        let f = forecast_eom(&fit, 30.0, 30.0);
        assert!((f.high - f.low).abs() < 1e-9);
    }
}
