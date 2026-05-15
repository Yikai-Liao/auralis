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
        "dcshift" => {
            let Some(shift) = params.get("shift") else {
                return Ok(vec![op.to_owned()]);
            };
            Ok(vec![op.to_owned(), param_as_string(shift, "shift")?])
        }
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
        "filter.highpass" => {
            let Some(cutoff) = params.get("cutoff") else {
                return Ok(vec![op.to_owned()]);
            };
            let cutoff = param_as_frequency_hz(cutoff, "cutoff")?;
            let mut tokens = vec!["highpass".to_owned(), cutoff];
            if let Some(q) = params.get("q") {
                tokens.push(format!("{}q", param_as_string(q, "q")?));
            }
            Ok(tokens)
        }
        "norm.peak" => {
            let Some(target) = params.get("target") else {
                return Ok(vec!["norm".to_owned()]);
            };
            Ok(vec!["norm".to_owned(), param_as_dbfs(target, "target")?])
        }
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
        _ => Ok(vec![op.to_owned()]),
    }
}

fn param_as_string(value: &toml::Value, param: &'static str) -> Result<String, EffectTokenError> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        _ => Err(invalid_param(param)),
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
