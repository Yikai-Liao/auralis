use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    CliError,
    command_args::{
        ChannelsArgs, ChorusArgs, ContrastArgs, DcShiftArgs, EchoArgs, FlangerArgs, GainArgs,
        NormArgs, OverdriveArgs, PhaserArgs, PipeArgs, RateArgs, RenderArgs, SaturationArgs,
        SimpleRecipeArgs, SoftVolArgs, SpeedArgs, TremoloArgs, TrimArgs, VolArgs,
    },
    command_support::{effect_input_to_chain_tokens, plan_graph_spec},
    graph_plan,
    plan_args::{PlanArgs, PlanCommand},
    recipe_args::{
        BandArgs, BandPassArgs, BandRejectArgs, BassArgs, EqualizerArgs, PitchArgs, PoleFilterArgs,
        TempoArgs, TrebleArgs,
    },
    recipes::{
        TimingArgs, band_effect_tokens, bandpass_effect_tokens, chorus_effect_tokens,
        echo_effect_tokens, optional_tail_effect_tokens, phaser_effect_tokens, pitch_effect_tokens,
        pole_filter_effect_tokens, saturation_effect_tokens, tempo_effect_tokens,
        vol_effect_tokens,
    },
    spec,
};

pub(super) fn run_plan_command(args: PlanArgs) -> Result<(), CliError> {
    let PlanArgs {
        spec,
        command,
        target,
        json,
        locked,
    } = args;

    if let Some(command) = command {
        return plan_modern_command(command, spec.as_ref(), target.as_ref(), locked, json);
    }

    let spec = spec.ok_or(CliError::MissingPlanInput)?;
    plan_graph_spec(&spec, target.as_deref(), json, locked)
}

fn plan_modern_command(
    command: PlanCommand,
    spec: Option<&PathBuf>,
    target: Option<&String>,
    locked: bool,
    json: bool,
) -> Result<(), CliError> {
    match command {
        PlanCommand::Render(render) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_render_command(render, json)
        }
        PlanCommand::Pipe(pipe) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_pipe_command(pipe, json)
        }
        recipe => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_recipe_surface_command(recipe, json)
        }
    }
}

fn plan_recipe_surface_command(command: PlanCommand, json: bool) -> Result<(), CliError> {
    match command {
        PlanCommand::Trim(trim) => plan_trim_command(trim, json),
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
        PlanCommand::Render(_) | PlanCommand::Pipe(_) => {
            unreachable!("render and pipe are handled before recipe planning")
        }
    }
}

fn plan_render_command(render: RenderArgs, json: bool) -> Result<(), CliError> {
    let checked = checked_render_spec(render)?;
    graph_plan::print_checked_plan("command:render", "render", &checked, None, json)
}

fn plan_pipe_command(pipe: PipeArgs, json: bool) -> Result<(), CliError> {
    let checked = checked_pipe_spec(pipe)?;
    graph_plan::print_checked_plan("command:pipe", "pipe", &checked, None, json)
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

fn reject_graph_plan_options(
    spec: Option<&PathBuf>,
    target: Option<&String>,
    locked: bool,
) -> Result<(), CliError> {
    if spec.is_some() || target.is_some() || locked {
        return Err(CliError::PlanCommandRejectsGraphOptions);
    }
    Ok(())
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

fn checked_render_spec(render: RenderArgs) -> Result<spec::CheckedGraphSpec, CliError> {
    let RenderArgs {
        input,
        output,
        backend: _,
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
        container,
        sample,
        effects_file,
        fx,
        chain,
    } = render;
    let mut sources = vec![spec::CheckedSource {
        id: "input".to_owned(),
        path: input,
    }];
    for (index, path) in additional_inputs.into_iter().enumerate() {
        sources.push(spec::CheckedSource {
            id: format!("input{}", index + 2),
            path,
        });
    }

    let mut nodes = Vec::new();
    let upstream = if sources.len() > 1 {
        nodes.push(spec::CheckedNode {
            id: "combine".to_owned(),
            op: Some(format!("combine.{}", combine.as_name())),
            inputs: sources
                .iter()
                .map(|source| format!("{}.audio", source.id))
                .collect(),
            input_gains: vec![None; sources.len()],
            params: BTreeMap::new(),
        });
        "combine.audio".to_owned()
    } else {
        "input.audio".to_owned()
    };

    let step_labels = render_step_labels(effects_file.as_ref(), &fx, chain.as_deref())?;
    let mut step_labels = step_labels;
    append_render_policy_steps(
        &mut step_labels,
        output_channels,
        no_auto_channels,
        output_sample_rate,
        no_auto_rate,
        guard,
        norm,
        dither,
        dither_seed,
        container,
        sample,
    );

    let (chains, sink_input, expanded_step_ids) = if step_labels.is_empty() {
        (Vec::new(), upstream, Vec::new())
    } else {
        let step_ids = step_labels
            .iter()
            .enumerate()
            .map(|(index, label)| format!("render/{:02}-{}", index + 1, step_slug(label)))
            .collect::<Vec<_>>();
        let chain = spec::CheckedChain {
            id: "render".to_owned(),
            input: upstream,
            step_ids: step_ids.clone(),
            step_labels,
            effect_tokens: Vec::new(),
        };
        (vec![chain], "render.audio".to_owned(), step_ids)
    };

    Ok(spec::CheckedGraphSpec {
        name: Some("render".to_owned()),
        source_count: sources.len(),
        chain_count: chains.len(),
        node_count: nodes.len(),
        sink_count: 1,
        sources,
        chains,
        nodes,
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: sink_input,
            path: output,
        }],
        expanded_step_ids,
    })
}

fn checked_recipe_spec<'a>(
    name: &str,
    input: PathBuf,
    output: PathBuf,
    tokens: impl IntoIterator<Item = &'a str>,
) -> Result<spec::CheckedGraphSpec, CliError> {
    let step_label = tokens.into_iter().collect::<Vec<_>>().join(" ");
    validate_effect_input(std::slice::from_ref(&step_label), None)?;
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

fn checked_pipe_spec(pipe: PipeArgs) -> Result<spec::CheckedGraphSpec, CliError> {
    let PipeArgs {
        input,
        expression,
        output,
        backend: _,
    } = pipe;
    validate_effect_input(&[], Some(expression.as_str()))?;
    let step_labels = expression
        .split('|')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let step_ids = step_labels
        .iter()
        .enumerate()
        .map(|(index, label)| format!("pipe/{:02}-{}", index + 1, step_slug(label)))
        .collect::<Vec<_>>();

    Ok(spec::CheckedGraphSpec {
        name: Some("pipe".to_owned()),
        source_count: 1,
        chain_count: 1,
        node_count: 0,
        sink_count: 1,
        sources: vec![spec::CheckedSource {
            id: "input".to_owned(),
            path: input,
        }],
        chains: vec![spec::CheckedChain {
            id: "pipe".to_owned(),
            input: "input.audio".to_owned(),
            step_ids: step_ids.clone(),
            step_labels,
            effect_tokens: Vec::new(),
        }],
        nodes: Vec::new(),
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: "pipe.audio".to_owned(),
            path: output,
        }],
        expanded_step_ids: step_ids,
    })
}

fn render_step_labels(
    effects_file: Option<&PathBuf>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<Vec<String>, CliError> {
    match (effects_file, !fx.is_empty(), chain) {
        (Some(path), false, None) => {
            auralis::parse_effects_file(path)?;
            Ok(vec![format!("effects-file {}", path.display())])
        }
        (None, true, None) => {
            validate_effect_input(fx, None)?;
            Ok(fx.to_vec())
        }
        (None, false, Some(chain)) => {
            validate_effect_input(&[], Some(chain))?;
            Ok(chain.split('|').map(str::trim).map(str::to_owned).collect())
        }
        (None, false, None) => Ok(Vec::new()),
        _ => Err(CliError::MixedEffectInputs),
    }
}

fn validate_effect_input(fx: &[String], chain: Option<&str>) -> Result<(), CliError> {
    let tokens = effect_input_to_chain_tokens(fx, chain)?;
    let token_refs = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    auralis::parse_effect_chain(&token_refs)?;
    Ok(())
}

#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn append_render_policy_steps(
    step_labels: &mut Vec<String>,
    output_channels: Option<auralis::ChannelCount>,
    no_auto_channels: bool,
    output_sample_rate: Option<auralis::SampleRate>,
    no_auto_rate: bool,
    guard: bool,
    norm: Option<f64>,
    dither: bool,
    dither_seed: Option<u32>,
    container: Option<crate::executor::OutputContainer>,
    sample: Option<auralis::WavSampleFormat>,
) {
    if let Some(channels) = output_channels {
        let suffix = if no_auto_channels { " no-auto" } else { "" };
        step_labels.push(format!("channels {}{}", channels.as_u16(), suffix));
    }
    if let Some(rate) = output_sample_rate {
        let suffix = if no_auto_rate { " no-auto" } else { "" };
        step_labels.push(format!("rate {}{}", rate.as_u32(), suffix));
    }
    if guard {
        step_labels.push("guard".to_owned());
    }
    if let Some(norm) = norm {
        step_labels.push(format!("norm {norm}"));
    }
    if dither {
        step_labels.push(match dither_seed {
            Some(seed) => format!("dither seed={seed}"),
            None => "dither".to_owned(),
        });
    }
    if let Some(container) = container {
        step_labels.push(format!("container {container:?}").to_lowercase());
    }
    if let Some(sample) = sample {
        step_labels.push(format!("sample {sample:?}").to_lowercase());
    }
}

fn step_slug(label: &str) -> String {
    label
        .chars()
        .filter_map(|value| {
            if value.is_ascii_alphanumeric() {
                Some(value.to_ascii_lowercase())
            } else if value.is_ascii_whitespace() || value == '-' || value == '_' {
                Some('-')
            } else {
                None
            }
        })
        .take(32)
        .collect()
}
