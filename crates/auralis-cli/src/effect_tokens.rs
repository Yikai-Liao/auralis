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
        "norm.peak" => {
            let Some(target) = params.get("target") else {
                return Ok(vec!["norm".to_owned()]);
            };
            Ok(vec!["norm".to_owned(), param_as_dbfs(target, "target")?])
        }
        "overdrive" => lower_ordered_tokens("overdrive", params, &["gain", "color"]),
        "repeat" => lower_ordered_tokens("repeat", params, &["count"]),
        "softvol" => {
            lower_ordered_tokens("softvol", params, &["volume", "double_time", "headroom"])
        }
        "speed" => lower_ordered_tokens("speed", params, &["factor"]),
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

fn param_as_string(value: &toml::Value, param: &'static str) -> Result<String, EffectTokenError> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        _ => Err(invalid_param(param)),
    }
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

fn invalid_param(param: &'static str) -> EffectTokenError {
    EffectTokenError { param }
}
