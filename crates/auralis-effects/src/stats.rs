use std::fmt::Write;

use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_WINDOW_TIME_SECONDS: f64 = 0.05;
const DEFAULT_SCALE: f64 = 1.0;
const MIN_BITS: u8 = 2;
const MAX_BITS: u8 = 32;
const MIN_WINDOW_TIME_SECONDS: f64 = 0.01;
const MAX_WINDOW_TIME_SECONDS: f64 = 10.0;
const MIN_SCALE: f64 = -99.0;
const MAX_SCALE: f64 = 99.0;

/// Display scaling mode for SoX-ng-style `stats` level fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatsDisplayScale {
    /// Render levels as floating-point values multiplied by the configured scale.
    Float(f64),
    /// Render levels as signed integer values using the configured bit depth.
    SignedBits(u8),
    /// Render levels as signed hexadecimal values using the configured bit depth.
    HexBits(u8),
}

/// SoX-ng-style `stats` analyzer configuration.
///
/// `stats` is an analysis effect: chain execution passes audio through
/// unchanged, while [`Self::report`] returns deterministic overall and
/// per-channel sample statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    display_scale: StatsDisplayScale,
    window_time_seconds: f64,
    json: bool,
}

impl Stats {
    /// Creates the default `stats` analyzer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            display_scale: StatsDisplayScale::Float(DEFAULT_SCALE),
            window_time_seconds: DEFAULT_WINDOW_TIME_SECONDS,
            json: false,
        }
    }

    /// Creates a `stats -s scale` analyzer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStats`] when `scale` is not finite or is
    /// outside SoX-ng's supported `-99..=99` range.
    pub fn with_scale(mut self, scale: f64) -> Result<Self> {
        if !scale.is_finite() || !(MIN_SCALE..=MAX_SCALE).contains(&scale) {
            return Err(EffectError::InvalidStats);
        }
        self.display_scale = StatsDisplayScale::Float(scale);
        Ok(self)
    }

    /// Creates a `stats -b bits` analyzer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStats`] when `bits` is outside
    /// SoX-ng's supported `2..=32` range.
    pub fn with_signed_bits(mut self, bits: u8) -> Result<Self> {
        self.display_scale = validate_bits(bits).map(StatsDisplayScale::SignedBits)?;
        Ok(self)
    }

    /// Creates a `stats -x bits` analyzer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStats`] when `bits` is outside
    /// SoX-ng's supported `2..=32` range.
    pub fn with_hex_bits(mut self, bits: u8) -> Result<Self> {
        self.display_scale = validate_bits(bits).map(StatsDisplayScale::HexBits)?;
        Ok(self)
    }

    /// Creates a `stats -w time` analyzer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStats`] when `seconds` is not finite or
    /// is outside SoX-ng's supported `0.01..=10` range.
    pub fn with_window_time(mut self, seconds: f64) -> Result<Self> {
        if !seconds.is_finite()
            || !(MIN_WINDOW_TIME_SECONDS..=MAX_WINDOW_TIME_SECONDS).contains(&seconds)
        {
            return Err(EffectError::InvalidStats);
        }
        self.window_time_seconds = seconds;
        Ok(self)
    }

    /// Enables `stats -j` JSON report rendering.
    #[must_use]
    pub const fn json(mut self) -> Self {
        self.json = true;
        self
    }

    /// Returns the configured display scaling mode.
    #[must_use]
    pub const fn display_scale(self) -> StatsDisplayScale {
        self.display_scale
    }

    /// Returns the moving RMS window size in seconds.
    #[must_use]
    pub const fn window_time_seconds(self) -> f64 {
        self.window_time_seconds
    }

    /// Returns whether `-j` rendering is enabled.
    #[must_use]
    pub const fn is_json(self) -> bool {
        self.json
    }

    /// Collects deterministic statistics from decoded audio.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidStats`] if the input contains non-finite
    /// samples or the buffer shape cannot be represented for report
    /// calculations.
    pub fn report(self, audio: &AudioBuffer) -> Result<StatsReport> {
        StatsReport::from_audio(self, audio)
    }

    /// Applies pass-through chain behavior.
    pub fn process_buffer(self, _audio: &mut AudioBuffer) {}
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic report emitted by the `stats` analyzer.
#[derive(Debug, Clone, PartialEq)]
pub struct StatsReport {
    channel_count: usize,
    num_samples: u64,
    length_seconds: f64,
    window_time_seconds: f64,
    overall: StatsSummary,
    channels: Vec<StatsSummary>,
}

impl StatsReport {
    fn from_audio(stats: Stats, audio: &AudioBuffer) -> Result<Self> {
        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidStats)?;
        let channel_count = audio.channels().as_usize();
        let sample_rate = f64::from(audio.spec().sample_rate().as_u32());
        let window_samples = window_sample_count(stats.window_time_seconds, sample_rate)?;

        let mut channels = Vec::with_capacity(channel_count);
        for channel in 0..channel_count {
            let mut samples = Vec::with_capacity(frames);
            for frame in 0..frames {
                let sample = f64::from(
                    audio
                        .sample(channel, frame)
                        .ok_or(EffectError::InvalidStats)?,
                );
                if !sample.is_finite() {
                    return Err(EffectError::InvalidStats);
                }
                samples.push(sample);
            }
            channels.push(StatsSummary::from_samples(&samples, window_samples));
        }

        let num_samples = audio.frames().as_u64();
        let overall = StatsSummary::combine(&channels);
        Ok(Self {
            channel_count,
            num_samples,
            length_seconds: f64_from_u64(num_samples) / sample_rate,
            window_time_seconds: stats.window_time_seconds,
            overall,
            channels,
        })
    }

    /// Returns the number of channels included in the report.
    #[must_use]
    pub const fn channel_count(&self) -> usize {
        self.channel_count
    }

    /// Returns the per-channel frame count, matching SoX-ng's `Num samples`.
    #[must_use]
    pub const fn num_samples(&self) -> u64 {
        self.num_samples
    }

    /// Returns the audio length in seconds.
    #[must_use]
    pub const fn length_seconds(&self) -> f64 {
        self.length_seconds
    }

    /// Returns the overall aggregate summary.
    #[must_use]
    pub const fn overall(&self) -> &StatsSummary {
        &self.overall
    }

    /// Returns one summary per channel.
    #[must_use]
    pub fn channels(&self) -> &[StatsSummary] {
        &self.channels
    }

    /// Renders the report in a stable SoX-ng-style plain text shape.
    #[must_use]
    pub fn render_text(&self, stats: Stats) -> String {
        let mut output = String::new();
        if self.channel_count == 2 {
            output.push_str("             Overall     Left      Right\n");
        } else if self.channel_count > 1 {
            output.push_str("             Overall");
            for channel in 0..self.channel_count {
                let _ = write!(output, "     Ch{:<3}", channel + 1);
            }
            output.push('\n');
        }
        self.write_level_row(
            &mut output,
            "DC offset ",
            |summary| summary.dc_offset,
            stats,
        );
        self.write_level_row(
            &mut output,
            "Min level ",
            |summary| summary.min_level,
            stats,
        );
        self.write_level_row(
            &mut output,
            "Max level ",
            |summary| summary.max_level,
            stats,
        );
        self.write_db_row(&mut output, "Pk lev dB ", |summary| summary.peak_level_db());
        self.write_db_row(&mut output, "RMS lev dB", |summary| summary.rms_level_db());
        self.write_db_row(&mut output, "RMS Pk dB ", |summary| summary.rms_peak_db());
        self.write_db_row(&mut output, "RMS Tr dB ", |summary| summary.rms_trough_db());
        self.write_number_row(&mut output, "Crest factor", |summary| {
            summary.crest_factor()
        });
        self.write_number_row(&mut output, "Flat factor", |summary| {
            summary.flat_factor_db()
        });
        self.write_count_row(&mut output, "Pk count   ", |summary| summary.peak_count);
        self.write_bit_depth_row(&mut output);
        let _ = writeln!(output, "Num samples{:9}", self.num_samples);
        let _ = writeln!(output, "Length s   {:9.3}", self.length_seconds);
        output.push_str("Scale max ");
        write_scaled_value(&mut output, stats.display_scale, 1.0);
        output.push('\n');
        let _ = writeln!(output, "Window s   {:9.3}", self.window_time_seconds);
        output
    }

    /// Renders the report as deterministic JSON text.
    #[must_use]
    pub fn render_json(&self) -> String {
        let mut output = String::new();
        output.push_str("{\n");
        let _ = writeln!(output, "  \"channel_count\": {},", self.channel_count);
        output.push_str("  \"overall\": ");
        self.overall.write_json(&mut output, 2, true);
        output.push_str("  \"channels\": [\n");
        for (index, channel) in self.channels.iter().enumerate() {
            output.push_str("    ");
            channel.write_json(&mut output, 4, index + 1 != self.channels.len());
        }
        output.push_str("  ],\n");
        let _ = writeln!(output, "  \"num_samples\": {},", self.num_samples);
        let _ = writeln!(
            output,
            "  \"length\": {},",
            json_number(self.length_seconds)
        );
        let _ = writeln!(
            output,
            "  \"window\": {}",
            json_number(self.window_time_seconds)
        );
        output.push_str("}\n");
        output
    }

    /// Renders according to `stats` command options.
    #[must_use]
    pub fn render_for(&self, stats: Stats) -> String {
        if stats.is_json() {
            self.render_json()
        } else {
            self.render_text(stats)
        }
    }

    fn write_level_row(
        &self,
        output: &mut String,
        label: &str,
        value: impl Fn(&StatsSummary) -> f64,
        stats: Stats,
    ) {
        output.push_str(label);
        write_scaled_value(output, stats.display_scale, value(&self.overall));
        for channel in &self.channels {
            write_scaled_value(output, stats.display_scale, value(channel));
        }
        output.push('\n');
    }

    fn write_db_row(
        &self,
        output: &mut String,
        label: &str,
        value: impl Fn(&StatsSummary) -> Option<f64>,
    ) {
        output.push_str(label);
        write_optional_db(output, value(&self.overall));
        for channel in &self.channels {
            write_optional_db(output, value(channel));
        }
        output.push('\n');
    }

    fn write_number_row(
        &self,
        output: &mut String,
        label: &str,
        value: impl Fn(&StatsSummary) -> f64,
    ) {
        output.push_str(label);
        let _ = write!(output, "{:10.2}", value(&self.overall));
        for channel in &self.channels {
            let _ = write!(output, "{:10.2}", value(channel));
        }
        output.push('\n');
    }

    fn write_count_row(
        &self,
        output: &mut String,
        label: &str,
        value: impl Fn(&StatsSummary) -> u64,
    ) {
        output.push_str(label);
        let _ = write!(output, "{:10}", value(&self.overall));
        for channel in &self.channels {
            let _ = write!(output, "{:10}", value(channel));
        }
        output.push('\n');
    }

    fn write_bit_depth_row(&self, output: &mut String) {
        output.push_str("Bit-depth ");
        write_bit_depth(output, self.overall.bit_depth);
        for channel in &self.channels {
            write_bit_depth(output, channel.bit_depth);
        }
        output.push('\n');
    }
}

/// Summary statistics for one channel or the overall aggregate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsSummary {
    dc_offset: f64,
    min_level: f64,
    max_level: f64,
    rms_level: f64,
    rms_peak: f64,
    rms_trough: f64,
    flat_factor: f64,
    peak_count: u64,
    bit_depth: (u8, u8),
}

impl StatsSummary {
    fn from_samples(samples: &[f64], window_samples: usize) -> Self {
        if samples.is_empty() {
            return Self::silent();
        }

        let mut min_level = samples[0];
        let mut max_level = samples[0];
        let mut sum = 0.0;
        let mut sum_squares = 0.0;
        let mut peak_count = 0_u64;
        let mut mask_lo = 0_u32;
        let mut mask_hi = 0_u32;

        for &sample in samples {
            min_level = min_level.min(sample);
            max_level = max_level.max(sample);
            sum += sample;
            sum_squares += sample * sample;
            update_bit_masks(sample, &mut mask_lo, &mut mask_hi);
        }

        for &sample in samples {
            if (sample - min_level).abs() <= f64::EPSILON
                || (sample - max_level).abs() <= f64::EPSILON
            {
                peak_count += 1;
            }
        }

        let count = f64_from_usize(samples.len());
        let (rms_peak, rms_trough) = moving_rms_extrema(samples, window_samples);
        let flat_factor = flat_factor(samples, min_level, max_level);
        Self {
            dc_offset: sum / count,
            min_level,
            max_level,
            rms_level: (sum_squares / count).sqrt(),
            rms_peak,
            rms_trough,
            flat_factor,
            peak_count,
            bit_depth: bit_depth(mask_lo, mask_hi),
        }
    }

    fn combine(channels: &[Self]) -> Self {
        if channels.is_empty() {
            return Self::silent();
        }

        let mut min_level = channels[0].min_level;
        let mut max_level = channels[0].max_level;
        let mut dc_offset = channels[0].dc_offset;
        let mut rms_squares = 0.0;
        let mut rms_peak = 0.0_f64;
        let mut rms_trough = f64::INFINITY;
        let mut flat_factor_sum = 0.0;
        let mut peak_count = 0_u64;
        let mut bit_depth = (32, 0);

        for channel in channels {
            min_level = min_level.min(channel.min_level);
            max_level = max_level.max(channel.max_level);
            if channel.dc_offset.abs() > dc_offset.abs() {
                dc_offset = channel.dc_offset;
            }
            rms_squares += channel.rms_level * channel.rms_level;
            rms_peak = rms_peak.max(channel.rms_peak);
            rms_trough = rms_trough.min(channel.rms_trough);
            flat_factor_sum += channel.flat_factor;
            peak_count = peak_count.saturating_add(channel.peak_count);
            bit_depth.0 = bit_depth.0.min(channel.bit_depth.0);
            bit_depth.1 = bit_depth.1.max(channel.bit_depth.1);
        }

        let channel_count = f64_from_usize(channels.len());
        Self {
            dc_offset,
            min_level,
            max_level,
            rms_level: (rms_squares / channel_count).sqrt(),
            rms_peak,
            rms_trough,
            flat_factor: flat_factor_sum / channel_count,
            peak_count,
            bit_depth,
        }
    }

    const fn silent() -> Self {
        Self {
            dc_offset: 0.0,
            min_level: 0.0,
            max_level: 0.0,
            rms_level: 0.0,
            rms_peak: 0.0,
            rms_trough: 0.0,
            flat_factor: 0.0,
            peak_count: 0,
            bit_depth: (0, 0),
        }
    }

    /// Returns the DC offset.
    #[must_use]
    pub const fn dc_offset(&self) -> f64 {
        self.dc_offset
    }

    /// Returns the minimum sample level.
    #[must_use]
    pub const fn min_level(&self) -> f64 {
        self.min_level
    }

    /// Returns the maximum sample level.
    #[must_use]
    pub const fn max_level(&self) -> f64 {
        self.max_level
    }

    /// Returns the linear RMS level.
    #[must_use]
    pub const fn rms_level(&self) -> f64 {
        self.rms_level
    }

    /// Returns the number of samples equal to the channel min or max level.
    #[must_use]
    pub const fn peak_count(&self) -> u64 {
        self.peak_count
    }

    /// Returns the detected low/high bit-depth pair.
    #[must_use]
    pub const fn bit_depth(&self) -> (u8, u8) {
        self.bit_depth
    }

    /// Returns the peak level in dBFS, if non-silent.
    #[must_use]
    pub fn peak_level_db(self) -> Option<f64> {
        linear_to_db(self.max_level.max(-self.min_level))
    }

    /// Returns the RMS level in dBFS, if non-silent.
    #[must_use]
    pub fn rms_level_db(self) -> Option<f64> {
        linear_to_db(self.rms_level)
    }

    /// Returns the moving RMS peak in dBFS, if non-silent.
    #[must_use]
    pub fn rms_peak_db(self) -> Option<f64> {
        linear_to_db(self.rms_peak)
    }

    /// Returns the moving RMS trough in dBFS, if non-silent.
    #[must_use]
    pub fn rms_trough_db(self) -> Option<f64> {
        linear_to_db(self.rms_trough)
    }

    /// Returns the crest factor.
    #[must_use]
    pub fn crest_factor(self) -> f64 {
        if self.rms_level > f64::EPSILON {
            self.max_level.max(-self.min_level) / self.rms_level
        } else {
            1.0
        }
    }

    /// Returns the flat-factor estimate in decibels.
    #[must_use]
    pub fn flat_factor_db(self) -> f64 {
        linear_to_db(self.flat_factor).unwrap_or(0.0)
    }

    fn write_json(self, output: &mut String, indent: usize, trailing_comma: bool) {
        let base = " ".repeat(indent);
        let field = " ".repeat(indent + 2);
        let comma = if trailing_comma { "," } else { "" };
        let _ = writeln!(output, "{{");
        let _ = writeln!(
            output,
            "{field}\"dc_offset\": {},",
            json_number(self.dc_offset)
        );
        let _ = writeln!(
            output,
            "{field}\"min_level\": {},",
            json_number(self.min_level)
        );
        let _ = writeln!(
            output,
            "{field}\"max_level\": {},",
            json_number(self.max_level)
        );
        write_optional_json(output, &field, "peak_level_db", self.peak_level_db());
        write_optional_json(output, &field, "rms_level_db", self.rms_level_db());
        write_optional_json(output, &field, "rms_peak_db", self.rms_peak_db());
        write_optional_json(output, &field, "rms_trough_db", self.rms_trough_db());
        let _ = writeln!(
            output,
            "{field}\"crest_factor\": {},",
            json_number(self.crest_factor())
        );
        let _ = writeln!(
            output,
            "{field}\"flat_factor\": {},",
            json_number(self.flat_factor_db())
        );
        let _ = writeln!(output, "{field}\"peak_count\": {},", self.peak_count);
        let _ = writeln!(
            output,
            "{field}\"bit_depth\": [{}, {}]",
            self.bit_depth.0, self.bit_depth.1
        );
        let _ = writeln!(output, "{base}}}{comma}");
    }
}

fn validate_bits(bits: u8) -> Result<u8> {
    if (MIN_BITS..=MAX_BITS).contains(&bits) {
        Ok(bits)
    } else {
        Err(EffectError::InvalidStats)
    }
}

fn window_sample_count(seconds: f64, sample_rate: f64) -> Result<usize> {
    let samples = (seconds * sample_rate).round().max(1.0);
    if samples > f64_from_usize(usize::MAX) {
        return Err(EffectError::InvalidStats);
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "window size is validated positive and within usize before conversion"
    )]
    Ok(samples as usize)
}

fn moving_rms_extrema(samples: &[f64], window_samples: usize) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }

    let window = window_samples.min(samples.len()).max(1);
    let inv_window = 1.0 / f64_from_usize(window);
    let mut sum_squares = samples[..window]
        .iter()
        .map(|sample| sample * sample)
        .sum::<f64>();
    let mut min_rms = (sum_squares * inv_window).sqrt();
    let mut max_rms = min_rms;

    for index in window..samples.len() {
        sum_squares += samples[index] * samples[index];
        sum_squares -= samples[index - window] * samples[index - window];
        let rms = (sum_squares * inv_window).sqrt();
        min_rms = min_rms.min(rms);
        max_rms = max_rms.max(rms);
    }

    (max_rms, min_rms)
}

fn flat_factor(samples: &[f64], min_level: f64, max_level: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let mut run_sum = 0.0;
    let mut current_value = samples[0];
    let mut current_run = 0_usize;
    for &sample in samples {
        if (sample - current_value).abs() <= f64::EPSILON {
            current_run += 1;
        } else {
            if is_extreme(current_value, min_level, max_level) {
                run_sum += f64_from_usize(current_run * current_run);
            }
            current_value = sample;
            current_run = 1;
        }
    }
    if is_extreme(current_value, min_level, max_level) {
        run_sum += f64_from_usize(current_run * current_run);
    }

    let peak_count = samples
        .iter()
        .filter(|&&sample| is_extreme(sample, min_level, max_level))
        .count();
    if peak_count == 0 {
        0.0
    } else {
        run_sum / f64_from_usize(peak_count)
    }
}

fn is_extreme(sample: f64, min_level: f64, max_level: f64) -> bool {
    (sample - min_level).abs() <= f64::EPSILON || (sample - max_level).abs() <= f64::EPSILON
}

fn update_bit_masks(sample: f64, mask_lo: &mut u32, mask_hi: &mut u32) {
    let integer = quantized_i32(sample);
    let bits = u32::from_ne_bytes(integer.to_ne_bytes());
    *mask_lo |= bits;
    *mask_hi |= if integer < 0 { !bits } else { bits };
}

fn bit_depth(mut mask_lo: u32, mut mask_hi: u32) -> (u8, u8) {
    let mut low = 32_u8;
    while low > 0 && mask_lo & 1 == 0 {
        low -= 1;
        mask_lo >>= 1;
    }

    let mut high = low;
    while high > 0 && (mask_hi << 1) & 0x8000_0000 == 0 {
        high -= 1;
        mask_hi <<= 1;
    }

    (high, low)
}

fn write_scaled_value(output: &mut String, scale: StatsDisplayScale, value: f64) {
    match scale {
        StatsDisplayScale::Float(scale) => {
            let precision = if scale.abs() < 10.0 { 6 } else { 5 };
            let _ = write!(output, " {:9.*}", precision, scale * value);
        }
        StatsDisplayScale::SignedBits(bits) => {
            let integer = quantize_to_bits(value, bits);
            let _ = write!(output, " {integer:9}");
        }
        StatsDisplayScale::HexBits(bits) => {
            let integer = quantize_to_bits(value, bits);
            if integer < 0 {
                let hex = format!("{:x}", integer.unsigned_abs());
                let _ = write!(output, " {:>width$}", format!("-{hex}"), width = 9);
            } else {
                let _ = write!(output, " {integer:9x}");
            }
        }
    }
}

fn write_optional_db(output: &mut String, value: Option<f64>) {
    match value {
        Some(value) => {
            let _ = write!(output, "{value:10.2}");
        }
        None => output.push_str("         -"),
    }
}

fn write_bit_depth(output: &mut String, bit_depth: (u8, u8)) {
    let _ = write!(output, "     {:>2}/{:<2}", bit_depth.0, bit_depth.1);
}

fn write_optional_json(output: &mut String, indent: &str, name: &str, value: Option<f64>) {
    match value {
        Some(value) => {
            let _ = writeln!(output, "{indent}\"{name}\": {},", json_number(value));
        }
        None => {
            let _ = writeln!(output, "{indent}\"{name}\": null,");
        }
    }
}

fn linear_to_db(value: f64) -> Option<f64> {
    (value > f64::EPSILON).then_some(20.0 * value.log10())
}

fn json_number(value: f64) -> String {
    if value.abs() <= f64::EPSILON {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "sample is clamped to i32 range before conversion"
)]
fn quantized_i32(sample: f64) -> i32 {
    let scaled = (sample.clamp(-1.0, 1.0) * f64::from(i32::MAX)).round();
    scaled.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "bits is validated to 2..=32 before calculating the signed scale"
)]
fn quantize_to_bits(sample: f64, bits: u8) -> i64 {
    let mult = 1_i64 << (bits - 1);
    let scaled = (sample * mult as f64).round();
    let max = (mult - 1) as f64;
    scaled.clamp(-(mult as f64), max) as i64
}

#[allow(
    clippy::cast_precision_loss,
    reason = "stats reports mirror SoX-ng's f64 summary calculations"
)]
fn f64_from_u64(value: u64) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_precision_loss,
    reason = "stats reports mirror SoX-ng's f64 summary calculations"
)]
fn f64_from_usize(value: usize) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    use super::{Stats, StatsDisplayScale};
    use crate::EffectError;

    #[test]
    fn report_includes_overall_and_per_channel_metrics() {
        let audio = audio_buffer(vec![0.5, -0.5, 0.25, -0.25], 2);
        let report = Stats::new().report(&audio).unwrap();

        assert_eq!(report.channel_count(), 2);
        assert_eq!(report.num_samples(), 2);
        assert!((report.length_seconds() - 2.0 / 48_000.0).abs() < 1.0e-12);
        assert!((report.overall().max_level() - 0.5).abs() < f64::EPSILON);
        assert!((report.overall().min_level() + 0.5).abs() < f64::EPSILON);
        assert_eq!(report.channels().len(), 2);
        assert_eq!(report.overall().peak_count(), 4);
    }

    #[test]
    fn options_validate_sox_ng_ranges_and_render_stable_shapes() {
        let audio = audio_buffer(vec![0.0, 0.5, -0.5, 0.25], 1);
        let stats = Stats::new()
            .with_signed_bits(16)
            .unwrap()
            .with_window_time(0.01)
            .unwrap();
        let report = stats.report(&audio).unwrap();

        assert_eq!(stats.display_scale(), StatsDisplayScale::SignedBits(16));
        assert!(report.render_text(stats).contains("Bit-depth"));
        assert!(report.render_text(stats).contains("Window s       0.010"));
        assert!(report.render_json().contains("\"channel_count\": 1"));
        assert_eq!(report.render_for(stats.json()), report.render_json());
    }

    #[test]
    fn rejects_invalid_options_and_non_finite_samples() {
        assert_eq!(
            Stats::new().with_signed_bits(1),
            Err(EffectError::InvalidStats)
        );
        assert_eq!(
            Stats::new().with_hex_bits(33),
            Err(EffectError::InvalidStats)
        );
        assert_eq!(
            Stats::new().with_window_time(0.001),
            Err(EffectError::InvalidStats)
        );
        assert_eq!(
            Stats::new().with_scale(100.0),
            Err(EffectError::InvalidStats)
        );
        assert_eq!(
            Stats::new().report(&audio_buffer(vec![f32::NAN], 1)),
            Err(EffectError::InvalidStats)
        );
    }

    fn audio_buffer(samples: Vec<f32>, channels: u16) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
            samples,
        )
        .unwrap()
    }
}
