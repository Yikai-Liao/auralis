use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    CliError, graph_plan,
    plan_args::PlanCommand,
    recipe_args::{ConcatArgs, MergeArgs, MixArgs, MixPowerArgs, MultiplyArgs},
    spec,
};

pub(super) fn plan_combine_surface_command(
    command: PlanCommand,
    json: bool,
) -> Result<(), CliError> {
    match command {
        PlanCommand::Mix(MixArgs {
            inputs,
            output,
            backend: _,
        }) => plan_combine_command("mix", auralis::CombineMethod::Mix, inputs, output, json),
        PlanCommand::Concat(ConcatArgs {
            inputs,
            output,
            backend: _,
        }) => plan_combine_command(
            "concat",
            auralis::CombineMethod::Concatenate,
            inputs,
            output,
            json,
        ),
        PlanCommand::MixPower(MixPowerArgs {
            inputs,
            output,
            backend: _,
        }) => plan_combine_command(
            "mix-power",
            auralis::CombineMethod::MixPower,
            inputs,
            output,
            json,
        ),
        PlanCommand::Merge(MergeArgs {
            inputs,
            output,
            backend: _,
        }) => plan_combine_command("merge", auralis::CombineMethod::Merge, inputs, output, json),
        PlanCommand::Multiply(MultiplyArgs {
            inputs,
            output,
            backend: _,
        }) => plan_combine_command(
            "multiply",
            auralis::CombineMethod::Multiply,
            inputs,
            output,
            json,
        ),
        _ => unreachable!("only combine recipe commands are handled here"),
    }
}

fn plan_combine_command(
    name: &str,
    combine: auralis::CombineMethod,
    inputs: Vec<PathBuf>,
    output: PathBuf,
    json: bool,
) -> Result<(), CliError> {
    let checked = checked_combine_spec(name, combine, inputs, output);
    graph_plan::print_checked_plan(&format!("command:{name}"), name, &checked, None, json)
}

fn checked_combine_spec(
    name: &str,
    combine: auralis::CombineMethod,
    inputs: Vec<PathBuf>,
    output: PathBuf,
) -> spec::CheckedGraphSpec {
    let sources = inputs
        .into_iter()
        .enumerate()
        .map(|(index, path)| spec::CheckedSource {
            id: if index == 0 {
                "input".to_owned()
            } else {
                format!("input{}", index + 1)
            },
            path,
        })
        .collect::<Vec<_>>();
    let source_count = sources.len();

    spec::CheckedGraphSpec {
        name: Some(name.to_owned()),
        source_count,
        chain_count: 0,
        node_count: 1,
        sink_count: 1,
        sources,
        chains: Vec::new(),
        nodes: vec![spec::CheckedNode {
            id: "combine".to_owned(),
            op: Some(format!("combine.{}", combine.as_name())),
            inputs: (0..source_count)
                .map(|index| {
                    if index == 0 {
                        "input.audio".to_owned()
                    } else {
                        format!("input{}.audio", index + 1)
                    }
                })
                .collect(),
            input_gains: vec![None; source_count],
            params: BTreeMap::default(),
        }],
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: "combine.audio".to_owned(),
            path: output,
        }],
        expanded_step_ids: Vec::new(),
    }
}
