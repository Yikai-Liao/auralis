use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectTokenError {
    param: &'static str,
}

impl EffectTokenError {
    pub fn param(&self) -> &'static str {
        self.param
    }
}

pub fn lower_graph_effect_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    match op {
        "allpass" => lower_pole_filter_tokens("allpass", params),
        "band" => lower_band_tokens(params),
        "bandpass" => lower_bandpass_tokens(params),
        "bandreject" => lower_ordered_tokens("bandreject", params, &["frequency", "width"]),
        "bass" => lower_ordered_tokens("bass", params, &["gain", "frequency", "width"]),
        "biquad" => lower_ordered_tokens("biquad", params, &["b0", "b1", "b2", "a0", "a1", "a2"]),
        "channels" => lower_ordered_tokens("channels", params, &["count"]),
        "chorus" => lower_chorus_tokens(params),
        "contrast" => lower_ordered_tokens("contrast", params, &["amount"]),
        "dcshift" => lower_ordered_tokens("dcshift", params, &["shift", "limiter_gain"]),
        "delay" => lower_repeated_tokens("delay", params, "positions"),
        "dither" => lower_dither_tokens(params),
        "downsample" => lower_ordered_tokens("downsample", params, &["factor"]),
        "echo" => lower_echo_tokens("echo", params),
        "echos" => lower_echo_tokens("echos", params),
        "equalizer" => lower_ordered_tokens("equalizer", params, &["frequency", "width", "gain"]),
        "gain" => {
            let Some(by) = params.get("by") else {
                return Ok(vec![op.to_owned()]);
            };
            Ok(vec![op.to_owned(), param_as_string(by, "by")?])
        }
        "fade" => {
            let Some(fade_in) = params.get("fade_in") else {
                return Ok(vec![op.to_owned()]);
            };
            let fade_in = param_as_string(fade_in, "fade_in")?;
            let curve = params
                .get("curve")
                .map(|value| param_as_string(value, "curve"))
                .transpose()?
                .map_or_else(|| "l".to_owned(), fade_curve_token);
            let Some(fade_out) = params.get("fade_out") else {
                return Ok(vec![op.to_owned(), curve, fade_in]);
            };
            let fade_out = param_as_string(fade_out, "fade_out")?;
            Ok(vec![
                op.to_owned(),
                curve,
                fade_in,
                "0".to_owned(),
                fade_out,
            ])
        }
        "filter.lowpass" | "lowpass" => lower_pole_filter_tokens("lowpass", params),
        "filter.highpass" | "highpass" => lower_pole_filter_tokens("highpass", params),
        "fir" => lower_source_or_repeated_tokens("fir", params, "source", "coefficients"),
        "firfit" => lower_source_or_repeated_tokens("firfit", params, "source", "knots"),
        "flanger" => lower_flanger_tokens(params),
        "hilbert" => lower_flagged_value_tokens("hilbert", params, "taps", "-n"),
        "loudness" => {
            lower_ordered_tokens("loudness", params, &["gain", "reference", "half_points"])
        }
        "norm.peak" => {
            let Some(target) = params.get("target") else {
                return Ok(vec!["norm".to_owned()]);
            };
            Ok(vec!["norm".to_owned(), param_as_dbfs(target, "target")?])
        }
        "overdrive" => lower_ordered_tokens("overdrive", params, &["gain", "color"]),
        "pad" => lower_pad_tokens(params),
        "pitch" => lower_pitch_tokens(params),
        "rate" => lower_rate_tokens(params),
        "repeat" => lower_ordered_tokens("repeat", params, &["count"]),
        "reverb" => lower_reverb_tokens(params),
        "remix" => lower_remix_tokens(params),
        "saturation" => lower_saturation_tokens(params),
        "sinc" => lower_sinc_tokens(params),
        "softvol" => {
            lower_ordered_tokens("softvol", params, &["volume", "double_time", "headroom"])
        }
        "speed" => lower_ordered_tokens("speed", params, &["factor"]),
        "stat" => lower_stat_tokens(params),
        "stats" => lower_stats_tokens(params),
        "stretch" => lower_stretch_tokens(params),
        "tempo" => lower_tempo_tokens(params),
        "treble" => lower_ordered_tokens("treble", params, &["gain", "frequency", "width"]),
        "tremolo" => lower_ordered_tokens("tremolo", params, &["speed", "depth"]),
        "trim" => {
            let Some(range) = params.get("range") else {
                return Ok(vec![op.to_owned()]);
            };
            let range = param_as_string(range, "range")?;
            let Some((start, end)) = range.split_once("..") else {
                return Err(invalid_param("range"));
            };
            Ok(vec![op.to_owned(), start.to_owned(), format!("={end}")])
        }
        "upsample" => lower_ordered_tokens("upsample", params, &["factor"]),
        "vol" => lower_ordered_tokens("vol", params, &["gain", "type", "limiter_gain"]),
        _ => Ok(vec![op.to_owned()]),
    }
}

fn lower_source_or_repeated_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
    source_param: &'static str,
    repeated_param: &'static str,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec![op.to_owned()];
    if let Some(source) = params.get(source_param) {
        tokens.push(param_as_string(source, source_param)?);
    } else if let Some(values) = params.get(repeated_param) {
        tokens.extend(param_as_string_array(values, repeated_param)?);
    }
    Ok(tokens)
}

fn lower_ordered_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
    ordered_params: &[&'static str],
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec![op.to_owned()];
    let mut missing_prefix = false;
    for param in ordered_params {
        let Some(value) = params.get(*param) else {
            missing_prefix = true;
            continue;
        };
        if missing_prefix {
            return Err(invalid_param(param));
        }
        tokens.push(param_as_string(value, param)?);
    }
    Ok(tokens)
}

fn lower_optional_ordered_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
    ordered_params: &[&'static str],
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec![op.to_owned()];
    append_optional_ordered(&mut tokens, params, ordered_params)?;
    Ok(tokens)
}

fn append_optional_ordered(
    tokens: &mut Vec<String>,
    params: &BTreeMap<String, toml::Value>,
    ordered_params: &[&'static str],
) -> Result<(), EffectTokenError> {
    let mut missing_prefix = false;
    for param in ordered_params {
        let Some(value) = params.get(*param) else {
            missing_prefix = true;
            continue;
        };
        if missing_prefix {
            return Err(invalid_param(param));
        }
        tokens.push(param_as_string(value, param)?);
    }
    Ok(())
}

fn lower_repeated_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
    param: &'static str,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(values) = params.get(param) else {
        return Ok(vec![op.to_owned()]);
    };
    let mut tokens = vec![op.to_owned()];
    tokens.extend(param_as_string_array(values, param)?);
    Ok(tokens)
}

fn lower_flagged_value_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
    param: &'static str,
    flag: &str,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(value) = params.get(param) else {
        return Ok(vec![op.to_owned()]);
    };
    Ok(vec![
        op.to_owned(),
        flag.to_owned(),
        param_as_string(value, param)?,
    ])
}

fn lower_pole_filter_tokens(
    effect: &str,
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(frequency) = params.get("frequency").or_else(|| params.get("cutoff")) else {
        return Ok(vec![effect.to_owned()]);
    };
    let mut tokens = vec![effect.to_owned()];
    if let Some(poles) = params.get("poles") {
        tokens.push(format!("-{}", param_as_string(poles, "poles")?));
    }
    tokens.push(param_as_frequency_hz(frequency, "frequency")?);
    if let Some(width) = params.get("width") {
        tokens.push(param_as_string(width, "width")?);
    } else if let Some(q) = params.get("q") {
        tokens.push(format!("{}q", param_as_string(q, "q")?));
    }
    Ok(tokens)
}

fn lower_band_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(frequency) = params.get("frequency") else {
        return Ok(vec!["band".to_owned()]);
    };
    let mut tokens = vec!["band".to_owned()];
    if param_as_bool(params.get("unpitched"), "unpitched")? {
        tokens.push("-n".to_owned());
    }
    tokens.push(param_as_frequency_hz(frequency, "frequency")?);
    if let Some(width) = params.get("width") {
        tokens.push(param_as_string(width, "width")?);
    }
    Ok(tokens)
}

fn lower_bandpass_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(frequency) = params.get("frequency") else {
        return Ok(vec!["bandpass".to_owned()]);
    };
    let mut tokens = vec!["bandpass".to_owned()];
    if param_as_bool(params.get("constant_skirt"), "constant_skirt")? {
        tokens.push("-c".to_owned());
    }
    tokens.push(param_as_frequency_hz(frequency, "frequency")?);
    if let Some(width) = params.get("width") {
        tokens.push(param_as_string(width, "width")?);
    }
    Ok(tokens)
}

fn lower_pad_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    if params.is_empty() {
        return Ok(vec!["pad".to_owned()]);
    }

    let mut tokens = vec!["pad".to_owned()];
    tokens.push(
        params
            .get("start")
            .map(|value| param_as_string(value, "start"))
            .transpose()?
            .unwrap_or_else(|| "0".to_owned()),
    );
    if let Some(positioned) = params.get("positioned") {
        tokens.extend(param_as_string_array(positioned, "positioned")?);
    }
    tokens.push(
        params
            .get("end")
            .map(|value| param_as_string(value, "end"))
            .transpose()?
            .unwrap_or_else(|| "0".to_owned()),
    );
    Ok(tokens)
}

fn lower_dither_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["dither".to_owned()];
    if params
        .get("noise_shape")
        .map(|value| param_as_string(value, "noise_shape"))
        .transpose()?
        .as_deref()
        == Some("shibata")
    {
        tokens.push("-s".to_owned());
    } else if param_as_bool(params.get("sloped"), "sloped")? {
        tokens.push("-S".to_owned());
    }
    if let Some(precision) = params.get("precision") {
        tokens.push("-p".to_owned());
        tokens.push(param_as_string(precision, "precision")?);
    }
    Ok(tokens)
}

fn lower_chorus_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["chorus".to_owned()];
    if let Some(interpolation) = params.get("interpolation") {
        push_interpolation_flag(
            &mut tokens,
            param_as_string(interpolation, "interpolation")?,
            true,
        );
    }
    if let Some(wave) = params.get("wave") {
        tokens.push(global_wave_flag(param_as_string(wave, "wave")?));
    }

    let Some(stages) = params.get("stages") else {
        append_optional_ordered(&mut tokens, params, &["gain_in", "gain_out"])?;
        return Ok(tokens);
    };

    tokens.push(
        params
            .get("gain_in")
            .map(|value| param_as_string(value, "gain_in"))
            .transpose()?
            .unwrap_or_else(|| "0.5".to_owned()),
    );
    tokens.push(
        params
            .get("gain_out")
            .map(|value| param_as_string(value, "gain_out"))
            .transpose()?
            .unwrap_or_else(|| "1".to_owned()),
    );
    append_chorus_stages(&mut tokens, stages)?;
    Ok(tokens)
}

fn lower_echo_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(taps) = params.get("taps") else {
        return lower_ordered_tokens(op, params, &["gain_in", "gain_out"]);
    };

    let mut tokens = lower_ordered_tokens(op, params, &["gain_in", "gain_out"])?;
    append_echo_taps(&mut tokens, taps)?;
    Ok(tokens)
}

fn lower_flanger_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["flanger".to_owned()];
    if let Some(interpolation) = params.get("interpolation") {
        push_interpolation_flag(
            &mut tokens,
            param_as_string(interpolation, "interpolation")?,
            true,
        );
    }
    let wave = params
        .get("wave")
        .map(|value| param_as_string(value, "wave"))
        .transpose()?;
    if matches!(wave.as_deref(), Some("triangle" | "t")) {
        tokens.push("-t".to_owned());
    }
    append_optional_ordered(
        &mut tokens,
        params,
        &["delay", "depth", "regen", "width", "speed"],
    )?;
    if let Some(wave) = wave {
        tokens.push(wave_token(wave));
    }
    if let Some(phase) = params.get("phase") {
        tokens.push(param_as_string(phase, "phase")?);
    }
    Ok(tokens)
}

fn append_echo_taps(tokens: &mut Vec<String>, value: &toml::Value) -> Result<(), EffectTokenError> {
    let toml::Value::Array(taps) = value else {
        return Err(invalid_param("taps"));
    };
    for tap in taps {
        let toml::Value::Table(tap) = tap else {
            return Err(invalid_param("taps"));
        };
        for param in ["delay", "decay"] {
            let Some(value) = tap.get(param) else {
                return Err(invalid_param("taps"));
            };
            tokens.push(param_as_string(value, "taps")?);
        }
    }
    Ok(())
}

fn append_chorus_stages(
    tokens: &mut Vec<String>,
    value: &toml::Value,
) -> Result<(), EffectTokenError> {
    let toml::Value::Array(stages) = value else {
        return Err(invalid_param("stages"));
    };
    for stage in stages {
        let toml::Value::Table(stage) = stage else {
            return Err(invalid_param("stages"));
        };
        for param in ["delay", "decay", "speed", "depth"] {
            let Some(value) = stage.get(param) else {
                return Err(invalid_param("stages"));
            };
            tokens.push(param_as_string(value, "stages")?);
        }
        if let Some(wave) = stage.get("wave") {
            tokens.push(stage_wave_flag(param_as_string(wave, "stages")?));
        }
    }
    Ok(())
}

fn lower_reverb_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["reverb".to_owned()];
    if param_as_bool(params.get("wet_only"), "wet_only")? {
        tokens.push("-w".to_owned());
    }
    append_optional_ordered(
        &mut tokens,
        params,
        &[
            "reverberance",
            "hf_damping",
            "room_scale",
            "stereo_depth",
            "pre_delay",
            "wet_gain",
        ],
    )?;
    Ok(tokens)
}

fn lower_saturation_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(saturation_type) = params.get("type") else {
        return Ok(vec!["saturation".to_owned()]);
    };
    let mut tokens = vec![
        "saturation".to_owned(),
        param_as_string(saturation_type, "type")?,
    ];
    append_optional_ordered(&mut tokens, params, &["blend", "offset", "parameter"])?;
    Ok(tokens)
}

fn lower_sinc_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["sinc".to_owned()];
    if let Some(beta) = params.get("beta") {
        tokens.push("-b".to_owned());
        tokens.push(param_as_string(beta, "beta")?);
    } else if let Some(attenuation) = params.get("attenuation") {
        tokens.push("-a".to_owned());
        tokens.push(param_as_string(attenuation, "attenuation")?);
    }
    if let Some(width) = params.get("transition_width") {
        tokens.push("-t".to_owned());
        tokens.push(param_as_string(width, "transition_width")?);
    }
    if let Some(taps) = params.get("taps") {
        tokens.push("-n".to_owned());
        tokens.push(param_as_string(taps, "taps")?);
    }
    if param_as_bool(params.get("round_taps"), "round_taps")? {
        tokens.push("-r".to_owned());
    }
    if let Some(range) = params.get("range") {
        tokens.push(param_as_string(range, "range")?);
    }
    if param_as_bool(params.get("delete_at_nyquist"), "delete_at_nyquist")? {
        tokens.push("-d".to_owned());
    }
    Ok(tokens)
}

fn lower_remix_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["remix".to_owned()];
    if let Some(level_mode) = params.get("level_mode") {
        match param_as_string(level_mode, "level_mode")?.as_str() {
            "automatic" => tokens.push("-a".to_owned()),
            "manual" => tokens.push("-m".to_owned()),
            "semi_automatic" => {}
            _ => return Err(invalid_param("level_mode")),
        }
    }
    if param_as_bool(params.get("mix_power"), "mix_power")? {
        tokens.push("-p".to_owned());
    }
    if let Some(outputs) = params.get("outputs") {
        tokens.extend(param_as_string_array(outputs, "outputs")?);
    }
    Ok(tokens)
}

fn lower_rate_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let Some(frequency) = params
        .get("frequency")
        .or_else(|| params.get("sample_rate"))
    else {
        return Ok(vec!["rate".to_owned()]);
    };
    let mut tokens = vec!["rate".to_owned()];
    if let Some(quality) = params.get("quality") {
        tokens.push(rate_quality_flag(param_as_string(quality, "quality")?));
    }
    tokens.push(param_as_string(frequency, "frequency")?);
    Ok(tokens)
}

fn lower_stat_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["stat".to_owned()];
    if let Some(scale) = params.get("scale") {
        tokens.push("-s".to_owned());
        tokens.push(param_as_string(scale, "scale")?);
    }
    if param_as_bool(params.get("rms"), "rms")? {
        tokens.push("-rms".to_owned());
    }
    if param_as_bool(params.get("volume_only"), "volume_only")? {
        tokens.push("-v".to_owned());
    }
    if param_as_bool(params.get("json"), "json")? {
        tokens.push("-j".to_owned());
    }
    Ok(tokens)
}

fn lower_stats_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["stats".to_owned()];
    if let Some(bits) = params.get("signed_bits") {
        tokens.push("-b".to_owned());
        tokens.push(param_as_string(bits, "signed_bits")?);
    } else if let Some(bits) = params.get("hex_bits") {
        tokens.push("-x".to_owned());
        tokens.push(param_as_string(bits, "hex_bits")?);
    } else if let Some(scale) = params.get("scale") {
        tokens.push("-s".to_owned());
        tokens.push(param_as_string(scale, "scale")?);
    }
    if let Some(window) = params.get("window") {
        tokens.push("-w".to_owned());
        tokens.push(param_as_string(window, "window")?);
    }
    if param_as_bool(params.get("json"), "json")? {
        tokens.push("-j".to_owned());
    }
    Ok(tokens)
}

fn lower_stretch_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = lower_optional_ordered_tokens("stretch", params, &["factor", "window"])?;
    if let Some(fade) = params.get("fade") {
        if tokens.len() < 3 {
            return Err(invalid_param("fade"));
        }
        tokens.push(stretch_fade_token(param_as_string(fade, "fade")?));
    } else if params.contains_key("shift") || params.contains_key("fading") {
        if tokens.len() < 3 {
            return Err(invalid_param("shift"));
        }
        tokens.push("l".to_owned());
    }
    append_optional_ordered(&mut tokens, params, &["shift", "fading"])?;
    Ok(tokens)
}

fn lower_tempo_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["tempo".to_owned()];
    if param_as_bool(params.get("quick"), "quick")? {
        tokens.push("-q".to_owned());
    }
    if let Some(profile) = params.get("profile") {
        tokens.push(profile_token(param_as_string(profile, "profile")?));
    }
    let Some(factor) = params.get("factor") else {
        return if tokens.len() == 1 {
            Ok(tokens)
        } else {
            Err(invalid_param("factor"))
        };
    };
    tokens.push(param_as_string(factor, "factor")?);
    append_optional_ordered(&mut tokens, params, &["segment", "search", "overlap"])?;
    Ok(tokens)
}

fn lower_pitch_tokens(
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, EffectTokenError> {
    let mut tokens = vec!["pitch".to_owned()];
    if param_as_bool(params.get("quick"), "quick")? {
        tokens.push("-q".to_owned());
    }
    let Some(cents) = params.get("cents") else {
        return if tokens.len() == 1 {
            Ok(tokens)
        } else {
            Err(invalid_param("cents"))
        };
    };
    tokens.push(param_as_string(cents, "cents")?);
    append_optional_ordered(&mut tokens, params, &["segment", "search", "overlap"])?;
    Ok(tokens)
}

fn param_as_string(value: &toml::Value, param: &'static str) -> Result<String, EffectTokenError> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        _ => Err(invalid_param(param)),
    }
}

fn param_as_string_array(
    value: &toml::Value,
    param: &'static str,
) -> Result<Vec<String>, EffectTokenError> {
    let toml::Value::Array(values) = value else {
        return Err(invalid_param(param));
    };
    values
        .iter()
        .map(|value| param_as_string(value, param))
        .collect()
}

fn param_as_bool(
    value: Option<&toml::Value>,
    param: &'static str,
) -> Result<bool, EffectTokenError> {
    match value {
        Some(toml::Value::Boolean(value)) => Ok(*value),
        Some(_) => Err(invalid_param(param)),
        None => Ok(false),
    }
}

fn param_as_frequency_hz(
    value: &toml::Value,
    param: &'static str,
) -> Result<String, EffectTokenError> {
    let value = param_as_string(value, param)?;
    Ok(value
        .strip_suffix("Hz")
        .or_else(|| value.strip_suffix("hz"))
        .unwrap_or(&value)
        .to_owned())
}

fn param_as_dbfs(value: &toml::Value, param: &'static str) -> Result<String, EffectTokenError> {
    let value = param_as_string(value, param)?;
    Ok(value
        .strip_suffix("dBFS")
        .or_else(|| value.strip_suffix("dbfs"))
        .or_else(|| value.strip_suffix("dB"))
        .or_else(|| value.strip_suffix("db"))
        .unwrap_or(&value)
        .to_owned())
}

fn fade_curve_token(value: String) -> String {
    match value.as_str() {
        "linear" => "t".to_owned(),
        "logarithmic" => "l".to_owned(),
        "quarter-sine" => "q".to_owned(),
        "half-sine" => "h".to_owned(),
        "inverted-parabola" => "p".to_owned(),
        _ => value,
    }
}

fn stretch_fade_token(value: String) -> String {
    match value.as_str() {
        "linear" => "l".to_owned(),
        "sqrt" => "s".to_owned(),
        "half" => "h".to_owned(),
        "quarter" => "q".to_owned(),
        _ => value,
    }
}

fn profile_token(value: String) -> String {
    match value.as_str() {
        "music" => "-m".to_owned(),
        "speech" => "-s".to_owned(),
        "linear" => "-l".to_owned(),
        _ => value,
    }
}

fn rate_quality_flag(value: String) -> String {
    match value.as_str() {
        "quick" | "q" => "-q".to_owned(),
        "low" | "l" => "-l".to_owned(),
        "medium" | "m" => "-m".to_owned(),
        "generic" | "g" => "-g".to_owned(),
        "high" | "h" => "-h".to_owned(),
        "extreme" | "e" => "-e".to_owned(),
        "very_high" | "very-high" | "v" => "-v".to_owned(),
        "ultra" | "u" => "-u".to_owned(),
        _ => value,
    }
}

fn push_interpolation_flag(tokens: &mut Vec<String>, value: String, flag_linear: bool) {
    match value.as_str() {
        "none" | "n" => tokens.push("-n".to_owned()),
        "linear" | "l" if flag_linear => tokens.push("-l".to_owned()),
        "linear" | "l" => {}
        "quadratic" | "q" => tokens.push("-q".to_owned()),
        _ => tokens.push(value),
    }
}

fn wave_token(value: String) -> String {
    match value.as_str() {
        "s" => "sine".to_owned(),
        "t" => "triangle".to_owned(),
        _ => value,
    }
}

fn global_wave_flag(value: String) -> String {
    match value.as_str() {
        "sine" | "s" => "-s".to_owned(),
        "triangle" | "t" => "-t".to_owned(),
        _ => value,
    }
}

fn stage_wave_flag(value: String) -> String {
    match value.as_str() {
        "sine" | "s" => "-sine".to_owned(),
        "triangle" | "t" => "-triangle".to_owned(),
        _ => value,
    }
}

fn invalid_param(param: &'static str) -> EffectTokenError {
    EffectTokenError { param }
}
