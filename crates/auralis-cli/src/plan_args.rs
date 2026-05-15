use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::{
    command_args::{
        ChannelsArgs, ChorusArgs, ContrastArgs, DcShiftArgs, EchoArgs, FlangerArgs, GainArgs,
        NormArgs, OverdriveArgs, PhaserArgs, PipeArgs, RateArgs, RenderArgs, SaturationArgs,
        SimpleRecipeArgs, SoftVolArgs, SpeedArgs, TremoloArgs, TrimArgs, VolArgs,
    },
    recipe_args::{
        BandArgs, BandPassArgs, BandRejectArgs, BassArgs, ConcatArgs, DelayArgs, DitherArgs,
        DownsampleArgs, EqualizerArgs, FadeArgs, HilbertArgs, LoudnessArgs, MergeArgs, MixArgs,
        MixPowerArgs, MultiplyArgs, PadArgs, PitchArgs, PoleFilterArgs, RepeatArgs, ReverbArgs,
        StretchArgs, TempoArgs, TrebleArgs, UpsampleArgs,
    },
};

#[derive(Debug, Args)]
pub(crate) struct PlanArgs {
    /// Auralis graph spec to plan.
    pub(crate) spec: Option<PathBuf>,

    /// Modern command surface to lower and plan.
    #[command(subcommand)]
    pub(crate) command: Option<PlanCommand>,

    /// Plan only the named target.
    #[arg(long, value_name = "TARGET")]
    pub(crate) target: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub(crate) json: bool,

    /// Require an up-to-date Auralis.lock before planning.
    #[arg(long)]
    pub(crate) locked: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum PlanCommand {
    /// Plan a linear render command without executing it.
    Render(RenderArgs),
    /// Plan a compact pipe command without executing it.
    Pipe(PipeArgs),
    /// Plan a trim recipe command without executing it.
    Trim(TrimArgs),
    /// Plan a gain recipe command without executing it.
    Gain(GainArgs),
    /// Plan a norm recipe command without executing it.
    Norm(NormArgs),
    /// Plan a rate recipe command without executing it.
    Rate(RateArgs),
    /// Plan a channels recipe command without executing it.
    Channels(ChannelsArgs),
    /// Plan a reverse recipe command without executing it.
    Reverse(SimpleRecipeArgs),
    /// Plan a deemph recipe command without executing it.
    Deemph(SimpleRecipeArgs),
    /// Plan an earwax recipe command without executing it.
    Earwax(SimpleRecipeArgs),
    /// Plan an echo recipe command without executing it.
    Echo(EchoArgs),
    /// Plan an echos recipe command without executing it.
    Echos(EchoArgs),
    /// Plan a chorus recipe command without executing it.
    Chorus(ChorusArgs),
    /// Plan a flanger recipe command without executing it.
    Flanger(FlangerArgs),
    /// Plan a phaser recipe command without executing it.
    Phaser(PhaserArgs),
    /// Plan an oops recipe command without executing it.
    Oops(SimpleRecipeArgs),
    /// Plan an RIAA recipe command without executing it.
    Riaa(SimpleRecipeArgs),
    /// Plan a swap recipe command without executing it.
    Swap(SimpleRecipeArgs),
    /// Plan a contrast recipe command without executing it.
    Contrast(ContrastArgs),
    /// Plan an overdrive recipe command without executing it.
    Overdrive(OverdriveArgs),
    /// Plan a saturation recipe command without executing it.
    Saturation(SaturationArgs),
    /// Plan a dcshift recipe command without executing it.
    #[command(name = "dcshift")]
    DcShift(DcShiftArgs),
    /// Plan a vol recipe command without executing it.
    Vol(VolArgs),
    /// Plan a softvol recipe command without executing it.
    #[command(name = "softvol")]
    SoftVol(SoftVolArgs),
    /// Plan a tremolo recipe command without executing it.
    Tremolo(TremoloArgs),
    /// Plan a speed recipe command without executing it.
    Speed(SpeedArgs),
    /// Plan a tempo recipe command without executing it.
    Tempo(TempoArgs),
    /// Plan a pitch recipe command without executing it.
    Pitch(PitchArgs),
    /// Plan a bass recipe command without executing it.
    Bass(BassArgs),
    /// Plan a treble recipe command without executing it.
    Treble(TrebleArgs),
    /// Plan an equalizer recipe command without executing it.
    Equalizer(EqualizerArgs),
    /// Plan an allpass recipe command without executing it.
    #[command(name = "allpass")]
    AllPass(PoleFilterArgs),
    /// Plan a band recipe command without executing it.
    Band(BandArgs),
    /// Plan a bandpass recipe command without executing it.
    #[command(name = "bandpass")]
    BandPass(BandPassArgs),
    /// Plan a bandreject recipe command without executing it.
    #[command(name = "bandreject")]
    BandReject(BandRejectArgs),
    /// Plan a highpass recipe command without executing it.
    #[command(name = "highpass")]
    HighPass(PoleFilterArgs),
    /// Plan a lowpass recipe command without executing it.
    #[command(name = "lowpass")]
    LowPass(PoleFilterArgs),
    /// Plan a fade recipe command without executing it.
    Fade(FadeArgs),
    /// Plan a delay recipe command without executing it.
    Delay(DelayArgs),
    /// Plan a pad recipe command without executing it.
    Pad(PadArgs),
    /// Plan a repeat recipe command without executing it.
    Repeat(RepeatArgs),
    /// Plan a downsample recipe command without executing it.
    Downsample(DownsampleArgs),
    /// Plan an upsample recipe command without executing it.
    Upsample(UpsampleArgs),
    /// Plan a hilbert recipe command without executing it.
    Hilbert(HilbertArgs),
    /// Plan a loudness recipe command without executing it.
    Loudness(LoudnessArgs),
    /// Plan a dither recipe command without executing it.
    Dither(DitherArgs),
    /// Plan a reverb recipe command without executing it.
    Reverb(ReverbArgs),
    /// Plan a stretch recipe command without executing it.
    Stretch(StretchArgs),
    /// Plan a mix recipe command without executing it.
    Mix(MixArgs),
    /// Plan a concat recipe command without executing it.
    Concat(ConcatArgs),
    /// Plan a mix-power recipe command without executing it.
    #[command(name = "mix-power")]
    MixPower(MixPowerArgs),
    /// Plan a merge recipe command without executing it.
    Merge(MergeArgs),
    /// Plan a multiply recipe command without executing it.
    Multiply(MultiplyArgs),
}
