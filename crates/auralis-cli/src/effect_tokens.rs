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
        "contrast" => lower_ordered_tokens("contrast", params, &["amount"]),
        "dcshift" => lower_ordered_tokens("dcshift", params, &["shift", "limiter_gain"]),
        "delay" => lower_repeated_tokens("delay", params, "positions"),
        "dither" => lower_dither_tokens(params),
        "downsample" => lower_ordered_tokens("downsample", params, &["factor"]),
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
        "repeat" => lower_ordered_tokens("repeat", params, &["count"]),
        "reverb" => lower_reverb_tokens(params),
        "softvol" => {
            lower_ordered_tokens("softvol", params, &["volume", "double_time", "headroom"])
        }
        "speed" => lower_ordered_tokens("speed", params, &["factor"]),
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
        _ => Ok(vec![op.to_owned()]),
    }
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

fn invalid_param(param: &'static str) -> EffectTokenError {
    EffectTokenError { param }
}
