use std::path::PathBuf;

use crate::{
    CliError,
    command_args::{
        ChannelsArgs, ChorusArgs, ContrastArgs, DcShiftArgs, EchoArgs, FlangerArgs, GainArgs,
        NormArgs, NormalizeArgs, OverdriveArgs, PhaserArgs, RateArgs, SaturationArgs,
        SimpleRecipeArgs, SoftVolArgs, SpeedArgs, TremoloArgs, TrimArgs, VolArgs,
    },
    graph_plan,
    plan_args::PlanCommand,
    plan_commands::step_slug,
    recipe_args::{
        BandArgs, BandPassArgs, BandRejectArgs, BassArgs, DelayArgs, DitherArgs, DownsampleArgs,
        EqualizerArgs, FadeArgs, HilbertArgs, LoudnessArgs, PadArgs, PitchArgs, PoleFilterArgs,
        RepeatArgs, ReverbArgs, StretchArgs, TempoArgs, TrebleArgs, UpsampleArgs,
    },
    recipes::{
        StretchRecipeOptions, TimingArgs, band_effect_tokens, bandpass_effect_tokens,
        chorus_effect_tokens, delay_effect_tokens, dither_effect_tokens, echo_effect_tokens,
        fade_effect_tokens, hilbert_effect_tokens, optional_tail_effect_tokens, pad_effect_tokens,
        phaser_effect_tokens, pitch_effect_tokens, pole_filter_effect_tokens, reverb_effect_tokens,
        saturation_effect_tokens, stretch_effect_tokens, tempo_effect_tokens, vol_effect_tokens,
    },
    spec,
};

pub(super) fn plan_recipe_surface_command(
    command: PlanCommand,
    json: bool,
) -> Result<(), CliError> {
    match command {
        PlanCommand::Trim(trim) => plan_trim_command(trim, json),
        PlanCommand::Normalize(normalize) => plan_normalize_command(normalize, json),
        PlanCommand::Gain(gain) => plan_gain_command(gain, json),
        PlanCommand::Norm(norm) => plan_norm_command(norm, json),
        PlanCommand::Rate(rate) => plan_rate_command(rate, json),
        PlanCommand::Channels(channels) => plan_channels_command(channels, json),
        PlanCommand::Reverse(reverse) => plan_reverse_command(reverse, json),
        PlanCommand::Deemph(deemph) => plan_simple_recipe_command("deemph", deemph, json),
        PlanCommand::Earwax(earwax) => plan_simple_recipe_command("earwax", earwax, json),
        PlanCommand::Echo(echo) => plan_echo_command("echo", echo, json),
        PlanCommand::Echos(echos) => plan_echo_command("echos", echos, json),
        PlanCommand::Chorus(chorus) => plan_chorus_command(chorus, json),
        PlanCommand::Flanger(flanger) => plan_flanger_command(flanger, json),
        PlanCommand::Phaser(phaser) => plan_phaser_command(phaser, json),
        PlanCommand::Oops(oops) => plan_simple_recipe_command("oops", oops, json),
        PlanCommand::Riaa(riaa) => plan_simple_recipe_command("riaa", riaa, json),
        PlanCommand::Swap(swap) => plan_simple_recipe_command("swap", swap, json),
        PlanCommand::Contrast(contrast) => plan_contrast_command(contrast, json),
        PlanCommand::Overdrive(overdrive) => plan_overdrive_command(overdrive, json),
        PlanCommand::Saturation(saturation) => plan_saturation_command(saturation, json),
        PlanCommand::DcShift(dcshift) => plan_dcshift_command(dcshift, json),
        PlanCommand::Vol(vol) => plan_vol_command(vol, json),
        PlanCommand::SoftVol(softvol) => plan_softvol_command(softvol, json),
        PlanCommand::Tremolo(tremolo) => plan_tremolo_command(tremolo, json),
        PlanCommand::Speed(speed) => plan_speed_command(speed, json),
        PlanCommand::Tempo(tempo) => plan_tempo_command(tempo, json),
        PlanCommand::Pitch(pitch) => plan_pitch_command(pitch, json),
        PlanCommand::Bass(bass) => plan_shelf_filter_command("bass", bass, json),
        PlanCommand::Treble(treble) => plan_treble_command(treble, json),
        PlanCommand::Equalizer(equalizer) => plan_equalizer_command(equalizer, json),
        PlanCommand::AllPass(allpass) => plan_pole_filter_command("allpass", allpass, json),
        PlanCommand::Band(band) => plan_band_command(band, json),
        PlanCommand::BandPass(bandpass) => plan_bandpass_command(bandpass, json),
        PlanCommand::BandReject(bandreject) => plan_bandreject_command(bandreject, json),
        PlanCommand::HighPass(highpass) => plan_pole_filter_command("highpass", highpass, json),
        PlanCommand::LowPass(lowpass) => plan_pole_filter_command("lowpass", lowpass, json),
        PlanCommand::Fade(fade) => plan_fade_command(fade, json),
        PlanCommand::Delay(delay) => plan_delay_command(delay, json),
        PlanCommand::Pad(pad) => plan_pad_command(pad, json),
        PlanCommand::Repeat(repeat) => plan_repeat_command(repeat, json),
        PlanCommand::Downsample(downsample) => plan_downsample_command(downsample, json),
        PlanCommand::Upsample(upsample) => plan_upsample_command(upsample, json),
        PlanCommand::Hilbert(hilbert) => plan_hilbert_command(hilbert, json),
        PlanCommand::Loudness(loudness) => plan_loudness_command(loudness, json),
        PlanCommand::Dither(dither) => plan_dither_command(dither, json),
        PlanCommand::Reverb(reverb) => plan_reverb_command(reverb, json),
        PlanCommand::Stretch(stretch) => plan_stretch_command(stretch, json),
        PlanCommand::Convert(_)
        | PlanCommand::Render(_)
        | PlanCommand::Pipe(_)
        | PlanCommand::Mix(_)
        | PlanCommand::Concat(_)
        | PlanCommand::MixPower(_)
        | PlanCommand::Merge(_)
        | PlanCommand::Multiply(_) => {
            unreachable!("non-effect recipes are handled before recipe planning")
        }
    }
}

fn plan_trim_command(trim: TrimArgs, json: bool) -> Result<(), CliError> {
    let TrimArgs {
        input,
        range,
        output,
        backend: _,
    } = trim;
    plan_recipe_command("trim", input, output, ["trim", range.as_str()], json)
}

fn plan_normalize_command(normalize: NormalizeArgs, json: bool) -> Result<(), CliError> {
    let NormalizeArgs {
        input,
        output,
        peak,
        backend: _,
    } = normalize;
    let peak = peak.to_string();
    plan_recipe_command("normalize", input, output, ["norm", peak.as_str()], json)
}

fn plan_gain_command(gain: GainArgs, json: bool) -> Result<(), CliError> {
    let GainArgs {
        input,
        db,
        output,
        backend: _,
    } = gain;
    plan_recipe_command("gain", input, output, ["gain", db.as_str()], json)
}

fn plan_norm_command(norm: NormArgs, json: bool) -> Result<(), CliError> {
    let NormArgs {
        input,
        level,
        output,
        backend: _,
    } = norm;
    plan_recipe_command("norm", input, output, ["norm", level.as_str()], json)
}

fn plan_rate_command(rate: RateArgs, json: bool) -> Result<(), CliError> {
    let RateArgs {
        input,
        sample_rate,
        output,
        backend: _,
    } = rate;
    plan_recipe_command("rate", input, output, ["rate", sample_rate.as_str()], json)
}

fn plan_channels_command(channels: ChannelsArgs, json: bool) -> Result<(), CliError> {
    let ChannelsArgs {
        input,
        count,
        output,
        backend: _,
    } = channels;
    plan_recipe_command(
        "channels",
        input,
        output,
        ["channels", count.as_str()],
        json,
    )
}

fn plan_reverse_command(reverse: SimpleRecipeArgs, json: bool) -> Result<(), CliError> {
    plan_simple_recipe_command("reverse", reverse, json)
}

fn plan_simple_recipe_command(
    name: &'static str,
    recipe: SimpleRecipeArgs,
    json: bool,
) -> Result<(), CliError> {
    let SimpleRecipeArgs {
        input,
        output,
        backend: _,
    } = recipe;
    plan_recipe_command(name, input, output, [name], json)
}

fn plan_echo_command(effect: &'static str, echo: EchoArgs, json: bool) -> Result<(), CliError> {
    let EchoArgs {
        input,
        gain_in,
        gain_out,
        taps,
        output,
        backend: _,
    } = echo;
    let tokens = echo_effect_tokens(effect, &gain_in, &gain_out, &taps);
    plan_recipe_command(
        effect,
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_chorus_command(chorus: ChorusArgs, json: bool) -> Result<(), CliError> {
    let ChorusArgs {
        input,
        gain_in,
        gain_out,
        interpolation,
        wave,
        stages,
        output,
        backend: _,
    } = chorus;
    let tokens = chorus_effect_tokens(&gain_in, &gain_out, &interpolation, &wave, &stages);
    plan_recipe_command(
        "chorus",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_flanger_command(flanger: FlangerArgs, json: bool) -> Result<(), CliError> {
    let FlangerArgs {
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
        backend: _,
    } = flanger;
    plan_recipe_command(
        "flanger",
        input,
        output,
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
        json,
    )
}

fn plan_phaser_command(phaser: PhaserArgs, json: bool) -> Result<(), CliError> {
    let PhaserArgs {
        input,
        gain_in,
        gain_out,
        delay,
        regen,
        speed,
        wave,
        interpolation,
        output,
        backend: _,
    } = phaser;
    let tokens = phaser_effect_tokens(
        &gain_in,
        &gain_out,
        &delay,
        &regen,
        &speed,
        &wave,
        &interpolation,
    );
    plan_recipe_command(
        "phaser",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_contrast_command(contrast: ContrastArgs, json: bool) -> Result<(), CliError> {
    let ContrastArgs {
        input,
        amount,
        output,
        backend: _,
    } = contrast;
    plan_recipe_command(
        "contrast",
        input,
        output,
        ["contrast", amount.as_str()],
        json,
    )
}

fn plan_overdrive_command(overdrive: OverdriveArgs, json: bool) -> Result<(), CliError> {
    let OverdriveArgs {
        input,
        gain,
        color,
        output,
        backend: _,
    } = overdrive;
    plan_recipe_command(
        "overdrive",
        input,
        output,
        ["overdrive", gain.as_str(), color.as_str()],
        json,
    )
}

fn plan_saturation_command(saturation: SaturationArgs, json: bool) -> Result<(), CliError> {
    let SaturationArgs {
        input,
        saturation_type,
        blend,
        offset,
        parameter,
        output,
        backend: _,
    } = saturation;
    let tokens = saturation_effect_tokens(&saturation_type, &blend, &offset, parameter.as_deref());
    plan_recipe_command(
        "saturation",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_dcshift_command(dcshift: DcShiftArgs, json: bool) -> Result<(), CliError> {
    let DcShiftArgs {
        input,
        shift,
        limiter_gain,
        output,
        backend: _,
    } = dcshift;
    let tokens = optional_tail_effect_tokens("dcshift", &shift, limiter_gain.as_deref());
    plan_recipe_command(
        "dcshift",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_vol_command(vol: VolArgs, json: bool) -> Result<(), CliError> {
    let VolArgs {
        input,
        gain,
        gain_type,
        limiter_gain,
        output,
        backend: _,
    } = vol;
    let tokens = vol_effect_tokens(&gain, gain_type.as_deref(), limiter_gain.as_deref());
    plan_recipe_command(
        "vol",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_softvol_command(softvol: SoftVolArgs, json: bool) -> Result<(), CliError> {
    let SoftVolArgs {
        input,
        volume,
        double_time,
        headroom,
        output,
        backend: _,
    } = softvol;
    plan_recipe_command(
        "softvol",
        input,
        output,
        [
            "softvol",
            volume.as_str(),
            double_time.as_str(),
            headroom.as_str(),
        ],
        json,
    )
}

fn plan_tremolo_command(tremolo: TremoloArgs, json: bool) -> Result<(), CliError> {
    let TremoloArgs {
        input,
        speed,
        depth,
        output,
        backend: _,
    } = tremolo;
    plan_recipe_command(
        "tremolo",
        input,
        output,
        ["tremolo", speed.as_str(), depth.as_str()],
        json,
    )
}

fn plan_speed_command(speed: SpeedArgs, json: bool) -> Result<(), CliError> {
    let SpeedArgs {
        input,
        factor,
        output,
        backend: _,
    } = speed;
    plan_recipe_command("speed", input, output, ["speed", factor.as_str()], json)
}

fn plan_tempo_command(tempo: TempoArgs, json: bool) -> Result<(), CliError> {
    let TempoArgs {
        input,
        factor,
        quick,
        profile,
        segment,
        search,
        overlap,
        output,
        backend: _,
    } = tempo;
    let tokens = tempo_effect_tokens(
        &factor,
        quick,
        profile.as_deref(),
        TimingArgs {
            segment: segment.as_deref(),
            search: search.as_deref(),
            overlap: overlap.as_deref(),
        },
    );
    plan_recipe_command(
        "tempo",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_pitch_command(pitch: PitchArgs, json: bool) -> Result<(), CliError> {
    let PitchArgs {
        input,
        cents,
        quick,
        segment,
        search,
        overlap,
        output,
        backend: _,
    } = pitch;
    let tokens = pitch_effect_tokens(
        &cents,
        quick,
        TimingArgs {
            segment: segment.as_deref(),
            search: search.as_deref(),
            overlap: overlap.as_deref(),
        },
    );
    plan_recipe_command(
        "pitch",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_shelf_filter_command(
    name: &'static str,
    shelf: BassArgs,
    json: bool,
) -> Result<(), CliError> {
    let BassArgs {
        input,
        gain,
        frequency,
        width,
        output,
        backend: _,
    } = shelf;
    plan_recipe_command(
        name,
        input,
        output,
        [name, gain.as_str(), frequency.as_str(), width.as_str()],
        json,
    )
}

fn plan_treble_command(treble: TrebleArgs, json: bool) -> Result<(), CliError> {
    let TrebleArgs {
        input,
        gain,
        frequency,
        width,
        output,
        backend: _,
    } = treble;
    plan_recipe_command(
        "treble",
        input,
        output,
        ["treble", gain.as_str(), frequency.as_str(), width.as_str()],
        json,
    )
}

fn plan_equalizer_command(equalizer: EqualizerArgs, json: bool) -> Result<(), CliError> {
    let EqualizerArgs {
        input,
        frequency,
        width,
        gain,
        output,
        backend: _,
    } = equalizer;
    plan_recipe_command(
        "equalizer",
        input,
        output,
        [
            "equalizer",
            frequency.as_str(),
            width.as_str(),
            gain.as_str(),
        ],
        json,
    )
}

fn plan_pole_filter_command(
    name: &'static str,
    filter: PoleFilterArgs,
    json: bool,
) -> Result<(), CliError> {
    let PoleFilterArgs {
        input,
        frequency,
        width,
        poles,
        output,
        backend: _,
    } = filter;
    let tokens = pole_filter_effect_tokens(name, &frequency, width.as_deref(), poles);
    plan_recipe_command(name, input, output, tokens.iter().map(String::as_str), json)
}

fn plan_band_command(band: BandArgs, json: bool) -> Result<(), CliError> {
    let BandArgs {
        input,
        frequency,
        width,
        unpitched,
        output,
        backend: _,
    } = band;
    let tokens = band_effect_tokens(&frequency, width.as_deref(), unpitched);
    plan_recipe_command(
        "band",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_bandpass_command(bandpass: BandPassArgs, json: bool) -> Result<(), CliError> {
    let BandPassArgs {
        input,
        frequency,
        width,
        constant_skirt,
        output,
        backend: _,
    } = bandpass;
    let tokens = bandpass_effect_tokens(&frequency, &width, constant_skirt);
    plan_recipe_command(
        "bandpass",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_bandreject_command(bandreject: BandRejectArgs, json: bool) -> Result<(), CliError> {
    let BandRejectArgs {
        input,
        frequency,
        width,
        output,
        backend: _,
    } = bandreject;
    plan_recipe_command(
        "bandreject",
        input,
        output,
        ["bandreject", frequency.as_str(), width.as_str()],
        json,
    )
}

fn plan_fade_command(fade: FadeArgs, json: bool) -> Result<(), CliError> {
    let FadeArgs {
        input,
        fade_in,
        fade_out,
        curve,
        output,
        backend: _,
    } = fade;
    let tokens = fade_effect_tokens(&fade_in, fade_out.as_deref(), &curve);
    plan_recipe_command(
        "fade",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_delay_command(delay: DelayArgs, json: bool) -> Result<(), CliError> {
    let DelayArgs {
        input,
        positions,
        output,
        backend: _,
    } = delay;
    let tokens = delay_effect_tokens(&positions);
    plan_recipe_command(
        "delay",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_pad_command(pad: PadArgs, json: bool) -> Result<(), CliError> {
    let PadArgs {
        input,
        start,
        end,
        positioned,
        output,
        backend: _,
    } = pad;
    let tokens = pad_effect_tokens(&start, &end, &positioned);
    plan_recipe_command(
        "pad",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_repeat_command(repeat: RepeatArgs, json: bool) -> Result<(), CliError> {
    let RepeatArgs {
        input,
        count,
        output,
        backend: _,
    } = repeat;
    plan_recipe_command("repeat", input, output, ["repeat", count.as_str()], json)
}

fn plan_downsample_command(downsample: DownsampleArgs, json: bool) -> Result<(), CliError> {
    let DownsampleArgs {
        input,
        factor,
        output,
        backend: _,
    } = downsample;
    plan_recipe_command(
        "downsample",
        input,
        output,
        ["downsample", factor.as_str()],
        json,
    )
}

fn plan_upsample_command(upsample: UpsampleArgs, json: bool) -> Result<(), CliError> {
    let UpsampleArgs {
        input,
        factor,
        output,
        backend: _,
    } = upsample;
    plan_recipe_command(
        "upsample",
        input,
        output,
        ["upsample", factor.as_str()],
        json,
    )
}

fn plan_hilbert_command(hilbert: HilbertArgs, json: bool) -> Result<(), CliError> {
    let HilbertArgs {
        input,
        taps,
        output,
        backend: _,
    } = hilbert;
    let tokens = hilbert_effect_tokens(taps.as_deref());
    plan_recipe_command(
        "hilbert",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_loudness_command(loudness: LoudnessArgs, json: bool) -> Result<(), CliError> {
    let LoudnessArgs {
        input,
        gain,
        reference,
        half_points,
        output,
        backend: _,
    } = loudness;
    plan_recipe_command(
        "loudness",
        input,
        output,
        [
            "loudness",
            gain.as_str(),
            reference.as_str(),
            half_points.as_str(),
        ],
        json,
    )
}

fn plan_dither_command(dither: DitherArgs, json: bool) -> Result<(), CliError> {
    let DitherArgs {
        input,
        sloped,
        noise_shape,
        precision,
        output,
        backend: _,
    } = dither;
    let tokens = dither_effect_tokens(sloped, noise_shape.as_deref(), &precision);
    plan_recipe_command(
        "dither",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_reverb_command(reverb: ReverbArgs, json: bool) -> Result<(), CliError> {
    let ReverbArgs {
        input,
        wet_only,
        reverberance,
        hf_damping,
        room_scale,
        stereo_depth,
        pre_delay,
        wet_gain,
        output,
        backend: _,
    } = reverb;
    let tokens = reverb_effect_tokens(
        wet_only,
        &reverberance,
        &hf_damping,
        &room_scale,
        &stereo_depth,
        &pre_delay,
        &wet_gain,
    );
    plan_recipe_command(
        "reverb",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_stretch_command(stretch: StretchArgs, json: bool) -> Result<(), CliError> {
    let StretchArgs {
        input,
        factor,
        window,
        fade,
        shift,
        fading,
        output,
        backend: _,
    } = stretch;
    let tokens = stretch_effect_tokens(StretchRecipeOptions {
        factor: &factor,
        window: &window,
        fade: &fade,
        shift: shift.as_deref(),
        fading: fading.as_deref(),
    });
    plan_recipe_command(
        "stretch",
        input,
        output,
        tokens.iter().map(String::as_str),
        json,
    )
}

fn plan_recipe_command<'a>(
    name: &str,
    input: PathBuf,
    output: PathBuf,
    tokens: impl IntoIterator<Item = &'a str>,
    json: bool,
) -> Result<(), CliError> {
    let checked = checked_recipe_spec(name, input, output, tokens)?;
    graph_plan::print_checked_plan(&format!("command:{name}"), name, &checked, None, json)
}

fn checked_recipe_spec<'a>(
    name: &str,
    input: PathBuf,
    output: PathBuf,
    tokens: impl IntoIterator<Item = &'a str>,
) -> Result<spec::CheckedGraphSpec, CliError> {
    let step_label = tokens.into_iter().collect::<Vec<_>>().join(" ");
    let token_refs = step_label.split_whitespace().collect::<Vec<_>>();
    auralis::parse_effect_chain(&token_refs)?;
    let step_id = format!("{name}/01-{}", step_slug(&step_label));

    Ok(spec::CheckedGraphSpec {
        name: Some(name.to_owned()),
        source_count: 1,
        chain_count: 1,
        node_count: 0,
        sink_count: 1,
        sources: vec![spec::CheckedSource {
            id: "input".to_owned(),
            path: input,
        }],
        chains: vec![spec::CheckedChain {
            id: name.to_owned(),
            input: "input.audio".to_owned(),
            step_ids: vec![step_id.clone()],
            step_labels: vec![step_label],
            effect_tokens: Vec::new(),
        }],
        nodes: Vec::new(),
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: format!("{name}.audio"),
            path: output,
        }],
        expanded_step_ids: vec![step_id],
    })
}
