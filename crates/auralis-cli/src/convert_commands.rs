use crate::{
    CliError,
    command_args::ConvertArgs,
    executor::{ConvertOptions, OutputGuard, convert_audio},
    spec,
};

pub(super) fn run_convert_command(args: &ConvertArgs) -> Result<(), CliError> {
    if args.export.is_some() {
        spec::validate_convert_graph_export(
            args.no_auto_channels,
            args.no_auto_rate,
            args.guard,
            args.container,
            args.sample,
        )?;
    }

    convert_audio(
        &args.input,
        &args.output,
        ConvertOptions {
            backend: args.backend,
            output_channels: args.output_channels,
            no_auto_channels: args.no_auto_channels,
            output_sample_rate: args.output_sample_rate,
            no_auto_rate: args.no_auto_rate,
            guard: OutputGuard::from(args.guard),
            norm: args.norm,
            container: args.container,
            sample: args.sample,
        },
    )?;

    if let Some(export) = &args.export {
        spec::write_convert_graph_spec(
            export,
            &args.input,
            &args.output,
            args.output_channels,
            args.output_sample_rate,
            args.norm,
        )?;
    }

    Ok(())
}
