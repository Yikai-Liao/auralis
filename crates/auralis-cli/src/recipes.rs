use std::path::{Path, PathBuf};

use crate::{
    CliError,
    executor::{
        ConvertOptions, OutputDither, OutputGuard, RenderOptions, convert_audio, run_pipeline,
    },
};

pub(super) fn run_trim_recipe(
    input: &Path,
    range: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    run_effect_recipe(input, output, backend, ["trim", range])
}

pub(super) fn run_fade_recipe(
    input: &Path,
    fade_in: &str,
    fade_out: Option<&str>,
    curve: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec![
        "fade".to_owned(),
        format!("in={fade_in}"),
        format!("curve={curve}"),
    ];
    if let Some(fade_out) = fade_out {
        effect_chain.push(format!("out={fade_out}"));
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_delay_recipe(
    input: &Path,
    positions: &[String],
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["delay".to_owned()];
    effect_chain.extend(positions.iter().cloned());

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_pad_recipe(
    input: &Path,
    start: &str,
    end: &str,
    positioned: &[String],
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["pad".to_owned(), start.to_owned()];
    effect_chain.extend(positioned.iter().cloned());
    effect_chain.push(end.to_owned());

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_saturation_recipe(
    input: &Path,
    saturation_type: &str,
    blend: &str,
    offset: &str,
    parameter: Option<&str>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec![
        "saturation".to_owned(),
        saturation_type.to_owned(),
        blend.to_owned(),
        offset.to_owned(),
    ];
    if let Some(parameter) = parameter {
        effect_chain.push(parameter.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_dc_shift_recipe(
    input: &Path,
    shift: &str,
    limiter_gain: Option<&str>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["dcshift".to_owned(), shift.to_owned()];
    if let Some(limiter_gain) = limiter_gain {
        effect_chain.push(limiter_gain.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_vol_recipe(
    input: &Path,
    gain: &str,
    gain_type: Option<&str>,
    limiter_gain: Option<&str>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["vol".to_owned(), gain.to_owned()];
    if let Some(gain_type) = gain_type {
        effect_chain.push(gain_type.to_owned());
    }
    if let Some(limiter_gain) = limiter_gain {
        effect_chain.push(limiter_gain.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_echo_recipe(
    effect: &str,
    input: &Path,
    gain_in: &str,
    gain_out: &str,
    taps: &[String],
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let effect_chain = echo_effect_tokens(effect, gain_in, gain_out, taps);

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn echo_effect_tokens(
    effect: &str,
    gain_in: &str,
    gain_out: &str,
    taps: &[String],
) -> Vec<String> {
    let mut effect_chain = vec![effect.to_owned(), gain_in.to_owned(), gain_out.to_owned()];
    for tap in taps {
        if let Some((delay, decay)) = tap.split_once(',') {
            effect_chain.push(delay.to_owned());
            effect_chain.push(decay.to_owned());
        } else {
            effect_chain.push(tap.to_owned());
        }
    }
    effect_chain
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_chorus_recipe(
    input: &Path,
    gain_in: &str,
    gain_out: &str,
    interpolation: &str,
    wave: &str,
    stages: &[String],
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["chorus".to_owned()];
    push_interpolation_flag(&mut effect_chain, interpolation);
    push_wave_flag(&mut effect_chain, wave);
    effect_chain.push(gain_in.to_owned());
    effect_chain.push(gain_out.to_owned());
    for stage in stages {
        for part in stage.split(',') {
            match part {
                "sine" => effect_chain.push("-sine".to_owned()),
                "triangle" => effect_chain.push("-triangle".to_owned()),
                other => effect_chain.push(other.to_owned()),
            }
        }
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_phaser_recipe(
    input: &Path,
    gain_in: &str,
    gain_out: &str,
    delay: &str,
    regen: &str,
    speed: &str,
    wave: &str,
    interpolation: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["phaser".to_owned()];
    push_interpolation_flag(&mut effect_chain, interpolation);
    push_wave_flag(&mut effect_chain, wave);
    effect_chain.extend([
        gain_in.to_owned(),
        gain_out.to_owned(),
        delay.to_owned(),
        regen.to_owned(),
        speed.to_owned(),
    ]);

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

fn push_interpolation_flag(effect_chain: &mut Vec<String>, interpolation: &str) {
    match interpolation {
        "none" => effect_chain.push("-n".to_owned()),
        "linear" => effect_chain.push("-l".to_owned()),
        "quadratic" => effect_chain.push("-q".to_owned()),
        other => effect_chain.push(other.to_owned()),
    }
}

fn push_wave_flag(effect_chain: &mut Vec<String>, wave: &str) {
    match wave {
        "sine" => effect_chain.push("-s".to_owned()),
        "triangle" => effect_chain.push("-t".to_owned()),
        other => effect_chain.push(other.to_owned()),
    }
}

#[derive(Clone, Copy)]
pub(super) struct TimingArgs<'a> {
    pub(super) segment: Option<&'a str>,
    pub(super) search: Option<&'a str>,
    pub(super) overlap: Option<&'a str>,
}

pub(super) fn run_tempo_recipe(
    input: &Path,
    factor: &str,
    quick: bool,
    profile: Option<&str>,
    timing: TimingArgs<'_>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["tempo".to_owned()];
    if quick {
        effect_chain.push("-q".to_owned());
    }
    if let Some(profile) = profile {
        effect_chain.push(match profile {
            "music" => "-m".to_owned(),
            "speech" => "-s".to_owned(),
            "linear" => "-l".to_owned(),
            _ => profile.to_owned(),
        });
    }
    effect_chain.push(factor.to_owned());
    push_timing_args(&mut effect_chain, timing);

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_pitch_recipe(
    input: &Path,
    cents: &str,
    quick: bool,
    timing: TimingArgs<'_>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["pitch".to_owned()];
    if quick {
        effect_chain.push("-q".to_owned());
    }
    effect_chain.push(cents.to_owned());
    push_timing_args(&mut effect_chain, timing);

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

fn push_timing_args(effect_chain: &mut Vec<String>, timing: TimingArgs<'_>) {
    if timing.segment.is_none() && timing.search.is_none() && timing.overlap.is_none() {
        return;
    }
    effect_chain.push(timing.segment.unwrap_or("0").to_owned());
    if timing.search.is_none() && timing.overlap.is_none() {
        return;
    }
    effect_chain.push(timing.search.unwrap_or("0").to_owned());
    if let Some(overlap) = timing.overlap {
        effect_chain.push(overlap.to_owned());
    }
}

pub(super) fn run_pole_filter_recipe(
    input: &Path,
    effect: &str,
    frequency: &str,
    width: Option<&str>,
    poles: Option<u8>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec![effect.to_owned()];
    if let Some(poles) = poles {
        effect_chain.push(format!("-{poles}"));
    }
    effect_chain.push(frequency.to_owned());
    if let Some(width) = width {
        effect_chain.push(width.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_band_recipe(
    input: &Path,
    frequency: &str,
    width: Option<&str>,
    unpitched: bool,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["band".to_owned()];
    if unpitched {
        effect_chain.push("-n".to_owned());
    }
    effect_chain.push(frequency.to_owned());
    if let Some(width) = width {
        effect_chain.push(width.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_bandpass_recipe(
    input: &Path,
    frequency: &str,
    width: &str,
    constant_skirt: bool,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["bandpass".to_owned()];
    if constant_skirt {
        effect_chain.push("-c".to_owned());
    }
    effect_chain.push(frequency.to_owned());
    effect_chain.push(width.to_owned());

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_hilbert_recipe(
    input: &Path,
    taps: Option<&str>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["hilbert".to_owned()];
    if let Some(taps) = taps {
        effect_chain.push("-n".to_owned());
        effect_chain.push(taps.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

pub(super) fn run_dither_recipe(
    input: &Path,
    sloped: bool,
    noise_shape: Option<&str>,
    precision: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["dither".to_owned()];
    if let Some("shibata") = noise_shape {
        effect_chain.push("-s".to_owned());
    } else if sloped {
        effect_chain.push("-S".to_owned());
    }
    effect_chain.push("-p".to_owned());
    effect_chain.push(precision.to_owned());

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_reverb_recipe(
    input: &Path,
    wet_only: bool,
    reverberance: &str,
    hf_damping: &str,
    room_scale: &str,
    stereo_depth: &str,
    pre_delay: &str,
    wet_gain: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec!["reverb".to_owned()];
    if wet_only {
        effect_chain.push("-w".to_owned());
    }
    effect_chain.extend([
        reverberance.to_owned(),
        hf_damping.to_owned(),
        room_scale.to_owned(),
        stereo_depth.to_owned(),
        pre_delay.to_owned(),
        wet_gain.to_owned(),
    ]);

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

#[derive(Clone, Copy)]
pub(super) struct StretchRecipeOptions<'a> {
    pub(super) factor: &'a str,
    pub(super) window: &'a str,
    pub(super) fade: &'a str,
    pub(super) shift: Option<&'a str>,
    pub(super) fading: Option<&'a str>,
}

pub(super) fn run_stretch_recipe(
    input: &Path,
    options: StretchRecipeOptions<'_>,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    let mut effect_chain = vec![
        "stretch".to_owned(),
        options.factor.to_owned(),
        options.window.to_owned(),
        stretch_fade_token(options.fade).to_owned(),
    ];
    if let Some(shift) = options.shift {
        effect_chain.push(shift.to_owned());
    }
    if let Some(fading) = options.fading {
        effect_chain.push(fading.to_owned());
    }

    run_effect_recipe(
        input,
        output,
        backend,
        effect_chain.iter().map(String::as_str),
    )
}

fn stretch_fade_token(fade: &str) -> &str {
    match fade {
        "linear" => "l",
        "sqrt" => "s",
        "half" => "h",
        "quarter" => "q",
        other => other,
    }
}

pub(super) fn run_effect_recipe<'a>(
    input: &Path,
    output: &Path,
    backend: auralis::BackendKind,
    effect_chain: impl IntoIterator<Item = &'a str>,
) -> Result<(), CliError> {
    let options = RenderOptions {
        backend,
        combine: auralis::CombineMethod::Concatenate,
        additional_inputs: Vec::new(),
        output_channels: None,
        no_auto_channels: false,
        output_sample_rate: None,
        no_auto_rate: false,
        guard: OutputGuard::Disabled,
        norm: None,
        dither: OutputDither::Disabled,
        dither_seed: None,
        effects_file: None,
        effect_chain: effect_chain.into_iter().map(ToOwned::to_owned).collect(),
        container: None,
        sample: None,
    };

    run_pipeline(input, output, &options)
}

pub(super) fn run_mix_recipe(
    inputs: &[PathBuf],
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    run_combine_recipe(inputs, output, backend, auralis::CombineMethod::Mix)
}

pub(super) fn run_combine_recipe(
    inputs: &[PathBuf],
    output: &Path,
    backend: auralis::BackendKind,
    combine: auralis::CombineMethod,
) -> Result<(), CliError> {
    let (input, additional_inputs) = inputs
        .split_first()
        .expect("clap requires at least two recipe inputs");
    let options = RenderOptions {
        backend,
        combine,
        additional_inputs: additional_inputs.to_vec(),
        output_channels: None,
        no_auto_channels: false,
        output_sample_rate: None,
        no_auto_rate: false,
        guard: OutputGuard::Disabled,
        norm: None,
        dither: OutputDither::Disabled,
        dither_seed: None,
        effects_file: None,
        effect_chain: Vec::new(),
        container: None,
        sample: None,
    };

    run_pipeline(input, output, &options)
}

pub(super) fn normalize_audio(
    input: &Path,
    output: &Path,
    peak: f64,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    convert_audio(
        input,
        output,
        ConvertOptions {
            backend,
            output_channels: None,
            no_auto_channels: false,
            output_sample_rate: None,
            no_auto_rate: false,
            guard: OutputGuard::Disabled,
            norm: Some(peak),
            container: None,
            sample: None,
        },
    )
}
