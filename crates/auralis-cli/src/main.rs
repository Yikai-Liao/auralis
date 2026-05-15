//! Auralis command-line entrypoint.

mod command_args;
mod command_support;
mod completions;
mod effect_tokens;
mod errors;
mod executor;
mod graph_commands;
mod graph_plan;
mod graph_runtime;
mod man_pages;
mod parsers;
mod recipe_args;
mod recipes;
mod spec;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

pub(crate) use command_args::GraphFormat;
use command_args::{
    ChannelsArgs, CheckArgs, ChorusArgs, CompletionsArgs, ContrastArgs, ConvertArgs, DcShiftArgs,
    EchoArgs, ExplainArgs, FlangerArgs, FmtArgs, GainArgs, GraphArgs, InitArgs, InspectArgs,
    ManArgs, NormArgs, NormalizeArgs, OpsArgs, OverdriveArgs, PhaserArgs, PipeArgs, PlanArgs,
    RateArgs, RenderArgs, RunArgs, SaturationArgs, SimpleRecipeArgs, SoftVolArgs, SpeedArgs,
    TremoloArgs, TrimArgs, VolArgs,
};
use command_support::{
    PathRole, check_command, effect_input_to_chain_tokens, init_project, inspect, plan_graph_spec,
    print_ops, run_graph_spec,
};
use completions::print_completions;
pub(crate) use errors::CliError;
use executor::{
    ConvertOptions, OutputDither, OutputGuard, RenderOptions, convert_audio, run_pipeline,
};
use graph_commands::{explain_graph_target, format_graph_spec, graph_spec};
use man_pages::print_man_page;
use recipe_args::{
    BandArgs, BandPassArgs, BandRejectArgs, BassArgs, ConcatArgs, DelayArgs, DitherArgs,
    DownsampleArgs, EqualizerArgs, FadeArgs, HilbertArgs, LoudnessArgs, MergeArgs, MixArgs,
    MixPowerArgs, MultiplyArgs, PadArgs, PitchArgs, PoleFilterArgs, RepeatArgs, ReverbArgs,
    StretchArgs, TempoArgs, TrebleArgs, UpsampleArgs,
};
use recipes::{
    StretchRecipeOptions, TimingArgs, normalize_audio, run_band_recipe, run_bandpass_recipe,
    run_chorus_recipe, run_combine_recipe, run_dc_shift_recipe, run_delay_recipe,
    run_dither_recipe, run_echo_recipe, run_effect_recipe, run_fade_recipe, run_hilbert_recipe,
    run_mix_recipe, run_pad_recipe, run_phaser_recipe, run_pitch_recipe, run_pole_filter_recipe,
    run_reverb_recipe, run_saturation_recipe, run_stretch_recipe, run_tempo_recipe,
    run_trim_recipe, run_vol_recipe,
};

#[derive(Debug, Parser)]
#[command(version, about = "Deterministic audio DSP tools.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Print metadata for a supported audio file.
    Inspect(InspectArgs),

    /// Convert one supported audio file into another container format.
    Convert(ConvertArgs),

    /// Keep one range from an audio file.
    Trim(TrimArgs),

    /// Normalize one audio file to a peak level.
    Normalize(NormalizeArgs),

    /// Normalize one audio file with the typed norm effect.
    Norm(NormArgs),

    /// Resample one audio file with the typed rate effect.
    Rate(RateArgs),

    /// Convert one audio file to a target channel count.
    Channels(ChannelsArgs),

    /// Adjust one audio file by a gain amount.
    Gain(GainArgs),

    /// Reverse one audio file.
    Reverse(SimpleRecipeArgs),

    /// Apply CD/DAT de-emphasis to one audio file.
    Deemph(SimpleRecipeArgs),

    /// Apply the stereo headphone-cue filter to one audio file.
    Earwax(SimpleRecipeArgs),

    /// Add one or more parallel delayed echoes.
    Echo(EchoArgs),

    /// Add one or more cascaded delayed echoes.
    Echos(EchoArgs),

    /// Add chorus modulation to one audio file.
    Chorus(ChorusArgs),

    /// Add flanger modulation to one audio file.
    Flanger(FlangerArgs),

    /// Add phaser modulation to one audio file.
    Phaser(PhaserArgs),

    /// Extract out-of-phase stereo content.
    Oops(SimpleRecipeArgs),

    /// Apply RIAA vinyl playback equalization.
    Riaa(SimpleRecipeArgs),

    /// Swap adjacent channel pairs.
    Swap(SimpleRecipeArgs),

    /// Enhance sample contrast.
    Contrast(ContrastArgs),

    /// Apply overdrive distortion.
    Overdrive(OverdriveArgs),

    /// Apply saturation distortion.
    Saturation(SaturationArgs),

    /// Shift the DC level of one audio file.
    #[command(name = "dcshift")]
    DcShift(DcShiftArgs),

    /// Apply SoX-ng volume scaling to one audio file.
    Vol(VolArgs),

    /// Apply soft volume changes to one audio file.
    #[command(name = "softvol")]
    SoftVol(SoftVolArgs),

    /// Apply tremolo modulation to one audio file.
    Tremolo(TremoloArgs),

    /// Change playback speed and sample rate.
    Speed(SpeedArgs),

    /// Change tempo without changing pitch.
    Tempo(TempoArgs),

    /// Shift pitch without changing tempo.
    Pitch(PitchArgs),

    /// Boost or cut bass frequencies in one audio file.
    Bass(BassArgs),

    /// Boost or cut treble frequencies in one audio file.
    Treble(TrebleArgs),

    /// Apply one peaking equalizer band to one audio file.
    Equalizer(EqualizerArgs),

    /// Apply an all-pass filter to one audio file.
    #[command(name = "allpass")]
    AllPass(PoleFilterArgs),

    /// Apply a resonator band-pass filter to one audio file.
    Band(BandArgs),

    /// Apply an RBJ band-pass filter to one audio file.
    #[command(name = "bandpass")]
    BandPass(BandPassArgs),

    /// Apply an RBJ band-reject filter to one audio file.
    #[command(name = "bandreject")]
    BandReject(BandRejectArgs),

    /// Apply a high-pass filter to one audio file.
    #[command(name = "highpass")]
    HighPass(PoleFilterArgs),

    /// Apply a low-pass filter to one audio file.
    #[command(name = "lowpass")]
    LowPass(PoleFilterArgs),

    /// Fade one audio file in or out.
    Fade(FadeArgs),

    /// Delay one audio file by per-channel positions.
    Delay(DelayArgs),

    /// Add silence before, after, or inside one audio file.
    Pad(PadArgs),

    /// Append finite copies of one audio file.
    Repeat(RepeatArgs),

    /// Keep every Nth sample from one audio file.
    Downsample(DownsampleArgs),

    /// Insert zero samples between input samples.
    Upsample(UpsampleArgs),

    /// Apply Hilbert transform phase shifting.
    Hilbert(HilbertArgs),

    /// Apply loudness compensation filtering.
    Loudness(LoudnessArgs),

    /// Apply deterministic dithering.
    Dither(DitherArgs),

    /// Apply stereo reverberation to one audio file.
    Reverb(ReverbArgs),

    /// Change duration with basic windowed stretching.
    Stretch(StretchArgs),

    /// Mix two or more audio files into one output.
    Mix(MixArgs),

    /// Concatenate two or more audio files end-to-end.
    Concat(ConcatArgs),

    /// Mix two or more audio files using equal-power scaling.
    MixPower(MixPowerArgs),

    /// Merge all channels from two or more audio files.
    Merge(MergeArgs),

    /// Multiply corresponding samples from two or more audio files.
    Multiply(MultiplyArgs),

    /// Render one ordered stream with typed effect syntax.
    Render(RenderArgs),

    /// Run one compact pipe-delimited DSP expression.
    Pipe(PipeArgs),

    /// Validate typed effect syntax without running audio processing.
    Check(CheckArgs),

    /// Preview the execution shape for an Auralis graph spec.
    Plan(PlanArgs),

    /// Emit an Auralis graph spec as a graph description.
    Graph(GraphArgs),

    /// Format an Auralis graph spec.
    Fmt(FmtArgs),

    /// Create an Auralis graph spec scaffold.
    Init(InitArgs),

    /// Generate shell completion scripts.
    Completions(CompletionsArgs),

    /// Print built-in manual pages for Auralis commands.
    Man(ManArgs),

    /// Explain how one graph node or target participates in execution.
    Explain(ExplainArgs),

    /// Run an Auralis graph spec.
    Run(RunArgs),

    /// List implemented typed effects or inspect one effect descriptor.
    Ops(OpsArgs),
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Inspect(InspectArgs { input, json }) => inspect(&input, json),
        Command::Convert(ConvertArgs {
            input,
            output,
            backend,
            output_channels,
            no_auto_channels,
            output_sample_rate,
            no_auto_rate,
            guard,
            norm,
            sample,
        }) => convert_audio(
            &input,
            &output,
            ConvertOptions {
                backend,
                output_channels,
                no_auto_channels,
                output_sample_rate,
                no_auto_rate,
                guard: OutputGuard::from(guard),
                norm,
                sample,
            },
        ),
        Command::Trim(TrimArgs {
            input,
            range,
            output,
            backend,
        }) => run_trim_recipe(&input, &range, &output, backend),
        Command::Normalize(NormalizeArgs {
            input,
            output,
            peak,
            backend,
        }) => normalize_audio(&input, &output, peak, backend),
        Command::Norm(NormArgs {
            input,
            level,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["norm", level.as_str()]),
        Command::Rate(RateArgs {
            input,
            sample_rate,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["rate", sample_rate.as_str()]),
        Command::Channels(ChannelsArgs {
            input,
            count,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["channels", count.as_str()]),
        Command::Gain(GainArgs {
            input,
            db,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["gain", db.as_str()]),
        Command::Reverse(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["reverse"]),
        Command::Deemph(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["deemph"]),
        Command::Earwax(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["earwax"]),
        Command::Echo(EchoArgs {
            input,
            gain_in,
            gain_out,
            taps,
            output,
            backend,
        }) => run_echo_recipe("echo", &input, &gain_in, &gain_out, &taps, &output, backend),
        Command::Echos(EchoArgs {
            input,
            gain_in,
            gain_out,
            taps,
            output,
            backend,
        }) => run_echo_recipe(
            "echos", &input, &gain_in, &gain_out, &taps, &output, backend,
        ),
        Command::Chorus(ChorusArgs {
            input,
            gain_in,
            gain_out,
            interpolation,
            wave,
            stages,
            output,
            backend,
        }) => run_chorus_recipe(
            &input,
            &gain_in,
            &gain_out,
            &interpolation,
            &wave,
            &stages,
            &output,
            backend,
        ),
        Command::Flanger(FlangerArgs {
            input,
            delay,
            depth,
            regen,
            width,
            speed,
            wave,
            phase,
            interpolation,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "flanger",
                delay.as_str(),
                depth.as_str(),
                regen.as_str(),
                width.as_str(),
                speed.as_str(),
                wave.as_str(),
                phase.as_str(),
                interpolation.as_str(),
            ],
        ),
        Command::Phaser(PhaserArgs {
            input,
            gain_in,
            gain_out,
            delay,
            regen,
            speed,
            wave,
            interpolation,
            output,
            backend,
        }) => run_phaser_recipe(
            &input,
            &gain_in,
            &gain_out,
            &delay,
            &regen,
            &speed,
            &wave,
            &interpolation,
            &output,
            backend,
        ),
        Command::Oops(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["oops"]),
        Command::Riaa(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["riaa"]),
        Command::Swap(SimpleRecipeArgs {
            input,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["swap"]),
        Command::Contrast(ContrastArgs {
            input,
            amount,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["contrast", amount.as_str()]),
        Command::Overdrive(OverdriveArgs {
            input,
            gain,
            color,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            ["overdrive", gain.as_str(), color.as_str()],
        ),
        Command::Saturation(SaturationArgs {
            input,
            saturation_type,
            blend,
            offset,
            parameter,
            output,
            backend,
        }) => run_saturation_recipe(
            &input,
            &saturation_type,
            &blend,
            &offset,
            parameter.as_deref(),
            &output,
            backend,
        ),
        Command::DcShift(DcShiftArgs {
            input,
            shift,
            limiter_gain,
            output,
            backend,
        }) => run_dc_shift_recipe(&input, &shift, limiter_gain.as_deref(), &output, backend),
        Command::Vol(VolArgs {
            input,
            gain,
            gain_type,
            limiter_gain,
            output,
            backend,
        }) => run_vol_recipe(
            &input,
            &gain,
            gain_type.as_deref(),
            limiter_gain.as_deref(),
            &output,
            backend,
        ),
        Command::SoftVol(SoftVolArgs {
            input,
            volume,
            double_time,
            headroom,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "softvol",
                volume.as_str(),
                double_time.as_str(),
                headroom.as_str(),
            ],
        ),
        Command::Tremolo(TremoloArgs {
            input,
            speed,
            depth,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            ["tremolo", speed.as_str(), depth.as_str()],
        ),
        Command::Speed(SpeedArgs {
            input,
            factor,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["speed", factor.as_str()]),
        Command::Tempo(TempoArgs {
            input,
            factor,
            quick,
            profile,
            segment,
            search,
            overlap,
            output,
            backend,
        }) => run_tempo_recipe(
            &input,
            &factor,
            quick,
            profile.as_deref(),
            TimingArgs {
                segment: segment.as_deref(),
                search: search.as_deref(),
                overlap: overlap.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Pitch(PitchArgs {
            input,
            cents,
            quick,
            segment,
            search,
            overlap,
            output,
            backend,
        }) => run_pitch_recipe(
            &input,
            &cents,
            quick,
            TimingArgs {
                segment: segment.as_deref(),
                search: search.as_deref(),
                overlap: overlap.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Bass(BassArgs {
            input,
            gain,
            frequency,
            width,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            ["bass", gain.as_str(), frequency.as_str(), width.as_str()],
        ),
        Command::Treble(TrebleArgs {
            input,
            gain,
            frequency,
            width,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            ["treble", gain.as_str(), frequency.as_str(), width.as_str()],
        ),
        Command::Equalizer(EqualizerArgs {
            input,
            frequency,
            width,
            gain,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "equalizer",
                frequency.as_str(),
                width.as_str(),
                gain.as_str(),
            ],
        ),
        Command::AllPass(PoleFilterArgs {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        }) => run_pole_filter_recipe(
            &input,
            "allpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::Band(BandArgs {
            input,
            frequency,
            width,
            unpitched,
            output,
            backend,
        }) => run_band_recipe(
            &input,
            &frequency,
            width.as_deref(),
            unpitched,
            &output,
            backend,
        ),
        Command::BandPass(BandPassArgs {
            input,
            frequency,
            width,
            constant_skirt,
            output,
            backend,
        }) => run_bandpass_recipe(&input, &frequency, &width, constant_skirt, &output, backend),
        Command::BandReject(BandRejectArgs {
            input,
            frequency,
            width,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            ["bandreject", frequency.as_str(), width.as_str()],
        ),
        Command::HighPass(PoleFilterArgs {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        }) => run_pole_filter_recipe(
            &input,
            "highpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::LowPass(PoleFilterArgs {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        }) => run_pole_filter_recipe(
            &input,
            "lowpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::Fade(FadeArgs {
            input,
            fade_in,
            fade_out,
            curve,
            output,
            backend,
        }) => run_fade_recipe(
            &input,
            &fade_in,
            fade_out.as_deref(),
            &curve,
            &output,
            backend,
        ),
        Command::Delay(DelayArgs {
            input,
            positions,
            output,
            backend,
        }) => run_delay_recipe(&input, &positions, &output, backend),
        Command::Pad(PadArgs {
            input,
            start,
            end,
            positioned,
            output,
            backend,
        }) => run_pad_recipe(&input, &start, &end, &positioned, &output, backend),
        Command::Repeat(RepeatArgs {
            input,
            count,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["repeat", count.as_str()]),
        Command::Downsample(DownsampleArgs {
            input,
            factor,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["downsample", factor.as_str()]),
        Command::Upsample(UpsampleArgs {
            input,
            factor,
            output,
            backend,
        }) => run_effect_recipe(&input, &output, backend, ["upsample", factor.as_str()]),
        Command::Hilbert(HilbertArgs {
            input,
            taps,
            output,
            backend,
        }) => run_hilbert_recipe(&input, taps.as_deref(), &output, backend),
        Command::Loudness(LoudnessArgs {
            input,
            gain,
            reference,
            half_points,
            output,
            backend,
        }) => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "loudness",
                gain.as_str(),
                reference.as_str(),
                half_points.as_str(),
            ],
        ),
        Command::Dither(DitherArgs {
            input,
            sloped,
            noise_shape,
            precision,
            output,
            backend,
        }) => run_dither_recipe(
            &input,
            sloped,
            noise_shape.as_deref(),
            &precision,
            &output,
            backend,
        ),
        Command::Reverb(ReverbArgs {
            input,
            wet_only,
            reverberance,
            hf_damping,
            room_scale,
            stereo_depth,
            pre_delay,
            wet_gain,
            output,
            backend,
        }) => run_reverb_recipe(
            &input,
            wet_only,
            &reverberance,
            &hf_damping,
            &room_scale,
            &stereo_depth,
            &pre_delay,
            &wet_gain,
            &output,
            backend,
        ),
        Command::Stretch(StretchArgs {
            input,
            factor,
            window,
            fade,
            shift,
            fading,
            output,
            backend,
        }) => run_stretch_recipe(
            &input,
            StretchRecipeOptions {
                factor: &factor,
                window: &window,
                fade: &fade,
                shift: shift.as_deref(),
                fading: fading.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Mix(MixArgs {
            inputs,
            output,
            backend,
        }) => run_mix_recipe(&inputs, &output, backend),
        Command::Concat(ConcatArgs {
            inputs,
            output,
            backend,
        }) => run_combine_recipe(
            &inputs,
            &output,
            backend,
            auralis::CombineMethod::Concatenate,
        ),
        Command::MixPower(MixPowerArgs {
            inputs,
            output,
            backend,
        }) => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::MixPower),
        Command::Merge(MergeArgs {
            inputs,
            output,
            backend,
        }) => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::Merge),
        Command::Multiply(MultiplyArgs {
            inputs,
            output,
            backend,
        }) => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::Multiply),
        Command::Render(RenderArgs {
            input,
            output,
            backend,
            combine,
            additional_inputs,
            output_channels,
            no_auto_channels,
            output_sample_rate,
            no_auto_rate,
            guard,
            norm,
            dither,
            dither_seed,
            effects_file,
            fx,
            chain,
        }) => {
            if effects_file.is_some() && (!fx.is_empty() || chain.is_some()) {
                return Err(CliError::MixedEffectInputs);
            }
            let options = RenderOptions {
                backend,
                combine,
                additional_inputs,
                output_channels,
                no_auto_channels,
                output_sample_rate,
                no_auto_rate,
                guard: OutputGuard::from(guard),
                norm,
                dither: OutputDither::from(dither),
                dither_seed,
                effects_file,
                effect_chain: effect_input_to_chain_tokens(&fx, chain.as_deref())?,
            };

            run_pipeline(&input, &output, &options)
        }
        Command::Pipe(PipeArgs {
            input,
            expression,
            output,
            backend,
        }) => {
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
                effect_chain: effect_input_to_chain_tokens(&[], Some(expression.as_str()))?,
            };

            run_pipeline(&input, &output, &options)
        }
        Command::Check(CheckArgs {
            spec,
            locked,
            effects_file,
            fx,
            chain,
        }) => check_command(
            spec.as_deref(),
            locked,
            effects_file.as_deref(),
            &fx,
            chain.as_deref(),
        ),
        Command::Plan(PlanArgs { spec, json, locked }) => plan_graph_spec(&spec, json, locked),
        Command::Graph(GraphArgs { spec, format }) => graph_spec(&spec, format),
        Command::Fmt(FmtArgs { spec, check }) => format_graph_spec(&spec, check),
        Command::Init(InitArgs { spec }) => init_project(&spec),
        Command::Completions(CompletionsArgs { shell }) => {
            print_completions(shell);
            Ok(())
        }
        Command::Man(ManArgs { topic }) => print_man_page(topic.as_deref()),
        Command::Explain(ExplainArgs { spec, target }) => explain_graph_target(&spec, &target),
        Command::Run(RunArgs { spec, locked }) => run_graph_spec(&spec, locked),
        Command::Ops(OpsArgs { effect, schema }) => print_ops(effect.as_deref(), schema),
    }
}
