#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectTokenError {
    param: &'static str,
}

impl EffectTokenError {
    pub fn param(&self) -> &'static str {
        self.param
    }
}

pub(super) fn param_as_string(
    value: &toml::Value,
    param: &'static str,
) -> Result<String, EffectTokenError> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        _ => Err(invalid_param(param)),
    }
}

pub(super) fn param_as_string_array(
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

pub(super) fn param_as_bool(
    value: Option<&toml::Value>,
    param: &'static str,
) -> Result<bool, EffectTokenError> {
    match value {
        Some(toml::Value::Boolean(value)) => Ok(*value),
        Some(_) => Err(invalid_param(param)),
        None => Ok(false),
    }
}

pub(super) fn param_as_frequency_hz(
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

pub(super) fn param_as_dbfs(
    value: &toml::Value,
    param: &'static str,
) -> Result<String, EffectTokenError> {
    let value = param_as_string(value, param)?;
    Ok(value
        .strip_suffix("dBFS")
        .or_else(|| value.strip_suffix("dbfs"))
        .or_else(|| value.strip_suffix("dB"))
        .or_else(|| value.strip_suffix("db"))
        .unwrap_or(&value)
        .to_owned())
}

pub(super) fn fade_curve_token(value: String) -> String {
    match value.as_str() {
        "linear" => "t".to_owned(),
        "logarithmic" => "l".to_owned(),
        "quarter-sine" => "q".to_owned(),
        "half-sine" => "h".to_owned(),
        "inverted-parabola" => "p".to_owned(),
        _ => value,
    }
}

pub(super) fn stretch_fade_token(value: String) -> String {
    match value.as_str() {
        "linear" => "l".to_owned(),
        "sqrt" => "s".to_owned(),
        "half" => "h".to_owned(),
        "quarter" => "q".to_owned(),
        _ => value,
    }
}

pub(super) fn splice_fade_token(value: String) -> String {
    match value.as_str() {
        "half_sine" | "half-sine" | "h" => "-h".to_owned(),
        "triangular" | "triangle" | "t" => "-t".to_owned(),
        "quarter_sine" | "quarter-sine" | "q" => "-q".to_owned(),
        _ => value,
    }
}

pub(super) fn profile_token(value: String) -> String {
    match value.as_str() {
        "music" => "-m".to_owned(),
        "speech" => "-s".to_owned(),
        "linear" => "-l".to_owned(),
        _ => value,
    }
}

pub(super) fn rate_quality_flag(value: String) -> String {
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

pub(super) fn push_interpolation_flag(tokens: &mut Vec<String>, value: String, flag_linear: bool) {
    match value.as_str() {
        "none" | "n" => tokens.push("-n".to_owned()),
        "linear" | "l" if flag_linear => tokens.push("-l".to_owned()),
        "linear" | "l" => {}
        "quadratic" | "q" => tokens.push("-q".to_owned()),
        _ => tokens.push(value),
    }
}

pub(super) fn wave_token(value: String) -> String {
    match value.as_str() {
        "s" => "sine".to_owned(),
        "t" => "triangle".to_owned(),
        _ => value,
    }
}

pub(super) fn global_wave_flag(value: String) -> String {
    match value.as_str() {
        "sine" | "s" => "-s".to_owned(),
        "triangle" | "t" => "-t".to_owned(),
        _ => value,
    }
}

pub(super) fn stage_wave_flag(value: String) -> String {
    match value.as_str() {
        "sine" | "s" => "-sine".to_owned(),
        "triangle" | "t" => "-triangle".to_owned(),
        _ => value,
    }
}

pub(super) fn invalid_param(param: &'static str) -> EffectTokenError {
    EffectTokenError { param }
}
