use crate::CliError;

struct ManPage {
    name: &'static str,
    summary: &'static str,
    synopsis: &'static str,
    description: &'static str,
    options: &'static [(&'static str, &'static str)],
}

const MAN_PAGES: &[ManPage] = &[
    ManPage {
        name: "auralis",
        summary: "modern deterministic audio processing CLI",
        synopsis: "auralis <command> [options]",
        description: "Auralis exposes conversion, ordered render pipelines, graph planning, graph execution, inspection, and developer tooling from one typed command surface.",
        options: &[
            ("inspect", "Print PCM16 WAV metadata, optionally as JSON."),
            (
                "convert",
                "Convert one supported audio file into another container format.",
            ),
            ("trim", "Keep one range from an audio file."),
            ("normalize", "Normalize one audio file to a peak level."),
            ("norm", "Normalize with the typed norm effect."),
            ("rate", "Resample with the typed rate effect."),
            ("channels", "Convert to a target channel count."),
            ("gain", "Adjust one audio file by a gain amount."),
            ("reverse", "Reverse one audio file."),
            ("deemph", "Apply CD/DAT de-emphasis to one audio file."),
            ("earwax", "Apply a stereo headphone-cue filter."),
            ("echo", "Add one or more parallel delayed echoes."),
            ("echos", "Add one or more cascaded delayed echoes."),
            ("chorus", "Add chorus modulation."),
            ("flanger", "Add flanger modulation."),
            ("phaser", "Add phaser modulation."),
            ("oops", "Extract out-of-phase stereo content."),
            ("riaa", "Apply RIAA vinyl playback equalization."),
            ("swap", "Swap adjacent channel pairs."),
            ("contrast", "Enhance sample contrast."),
            ("overdrive", "Apply overdrive distortion."),
            ("saturation", "Apply saturation distortion."),
            ("dcshift", "Shift DC level."),
            ("vol", "Apply SoX-ng volume scaling."),
            ("softvol", "Apply soft volume changes."),
            ("tremolo", "Apply tremolo modulation."),
            ("speed", "Change playback speed and sample rate."),
            ("tempo", "Change tempo without changing pitch."),
            ("pitch", "Shift pitch without changing tempo."),
            ("bass", "Boost or cut bass frequencies."),
            ("treble", "Boost or cut treble frequencies."),
            ("equalizer", "Apply one peaking equalizer band."),
            ("allpass", "Apply an all-pass filter."),
            ("band", "Apply a resonator band-pass filter."),
            ("bandpass", "Apply an RBJ band-pass filter."),
            ("bandreject", "Apply an RBJ band-reject filter."),
            ("highpass", "Apply a high-pass filter."),
            ("lowpass", "Apply a low-pass filter."),
            ("fade", "Fade one audio file in or out."),
            ("delay", "Delay audio channels."),
            ("pad", "Add silence padding."),
            ("repeat", "Append finite copies."),
            ("downsample", "Keep every Nth sample."),
            ("upsample", "Insert zero samples between input samples."),
            ("hilbert", "Apply Hilbert transform phase shifting."),
            ("loudness", "Apply loudness compensation filtering."),
            ("dither", "Apply deterministic dithering."),
            ("reverb", "Apply stereo reverberation."),
            ("stretch", "Change duration with windowed stretching."),
            ("mix", "Mix two or more audio files into one output."),
            ("concat", "Concatenate two or more audio files end-to-end."),
            (
                "mix-power",
                "Mix two or more audio files with equal-power scaling.",
            ),
            ("merge", "Merge channels from two or more audio files."),
            (
                "multiply",
                "Multiply corresponding samples from two or more audio files.",
            ),
            (
                "render",
                "Run one ordered DSP pipeline over one combined input stream.",
            ),
            ("pipe", "Run one compact DSP expression."),
            ("check", "Validate graph specs or typed effect syntax."),
            ("plan", "Preview execution shape for an Auralis graph spec."),
            (
                "graph",
                "Emit an Auralis graph as mermaid, dot, svg, or json.",
            ),
            ("fmt", "Format an Auralis graph spec."),
            ("init", "Create an Auralis graph spec scaffold."),
            ("cache", "Inspect or clear local persistent cache state."),
            ("completions", "Generate shell completion scripts."),
            ("man", "Print built-in manual pages."),
            (
                "explain",
                "Explain why a node or target participates in execution.",
            ),
            ("run", "Run an Auralis graph spec."),
            ("ops", "Inspect the typed operation registry."),
        ],
    },
    ManPage {
        name: "trim",
        summary: "keep one audio range",
        synopsis: "auralis trim INPUT.wav RANGE -o OUTPUT.wav [--backend BACKEND]",
        description: "Trim is a recipe alias for keeping one contiguous range. It lowers to the same typed effect pipeline as `render --fx 'trim ...'`.",
        options: &[
            (
                "RANGE",
                "Frame range to keep, for example `10..30` or `10..`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "normalize",
        summary: "normalize to a peak level",
        synopsis: "auralis normalize INPUT.wav -o OUTPUT.wav [--peak DBFS] [--backend BACKEND]",
        description: "Normalize is a recipe alias for output-boundary peak normalization. It lowers to the same output policy as `render --norm`.",
        options: &[
            ("-o, --output FILE", "Output audio file to create."),
            (
                "--peak DBFS",
                "Peak target in dBFS, accepting values like `-1` or `-1dBFS`.",
            ),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "norm",
        summary: "normalize with the typed norm effect",
        synopsis: "auralis norm INPUT.wav [DBFS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Norm is a recipe alias for the typed peak-normalization effect. It lowers to the same typed effect pipeline as `render --fx 'norm ...'`.",
        options: &[
            ("DBFS", "Peak target in dBFS."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "rate",
        summary: "resample with the typed rate effect",
        synopsis: "auralis rate INPUT.wav RATE -o OUTPUT.wav [--backend BACKEND]",
        description: "Rate is a recipe alias for typed sample-rate conversion. It lowers to the same typed effect pipeline as `render --fx 'rate ...'`.",
        options: &[
            ("RATE", "Target sample rate in Hz."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "channels",
        summary: "convert to a target channel count",
        synopsis: "auralis channels INPUT.wav CHANNELS -o OUTPUT.wav [--backend BACKEND]",
        description: "Channels is a recipe alias for typed channel-count conversion. It lowers to the same typed effect pipeline as `render --fx 'channels ...'`.",
        options: &[
            ("CHANNELS", "Target channel count."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "gain",
        summary: "adjust one audio file by gain",
        synopsis: "auralis gain INPUT.wav DB -o OUTPUT.wav [--backend BACKEND]",
        description: "Gain is a recipe alias for a single gain adjustment. It lowers to the same typed effect pipeline as `render --fx 'gain ...'`.",
        options: &[
            (
                "DB",
                "Gain adjustment in dB, accepting values like `-3` or `-3dB`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "reverse",
        summary: "reverse one audio file",
        synopsis: "auralis reverse INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Reverse is a recipe alias for reversing all frames in one input. It lowers to the same typed effect pipeline as `render --fx reverse`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "deemph",
        summary: "apply de-emphasis",
        synopsis: "auralis deemph INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Deemph is a recipe alias for CD/DAT de-emphasis. It lowers to the same typed effect pipeline as `render --fx deemph`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "earwax",
        summary: "apply headphone-cue filtering",
        synopsis: "auralis earwax INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Earwax is a recipe alias for the stereo headphone-cue filter. It lowers to the same typed effect pipeline as `render --fx earwax`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "echo",
        summary: "add parallel delayed echoes",
        synopsis: "auralis echo INPUT.wav [--gain-in GAIN] [--gain-out GAIN] --tap DELAY_MS,DECAY... -o OUTPUT.wav [--backend BACKEND]",
        description: "Echo is a recipe alias for one or more parallel delay taps. It lowers to the same typed effect pipeline as `render --fx 'echo ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--tap DELAY_MS,DECAY",
                "Echo tap delay in milliseconds and decay; repeat for multiple taps.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "echos",
        summary: "add cascaded delayed echoes",
        synopsis: "auralis echos INPUT.wav [--gain-in GAIN] [--gain-out GAIN] --tap DELAY_MS,DECAY... -o OUTPUT.wav [--backend BACKEND]",
        description: "Echos is a recipe alias for one or more cascaded delay taps. It lowers to the same typed effect pipeline as `render --fx 'echos ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--tap DELAY_MS,DECAY",
                "Echo tap delay in milliseconds and decay; repeat for multiple taps.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "chorus",
        summary: "add chorus modulation",
        synopsis: "auralis chorus INPUT.wav [--gain-in GAIN] [--gain-out GAIN] [--interpolation MODE] [--wave WAVE] [--stage DELAY_MS,DECAY,SPEED_HZ,DEPTH_MS[,WAVE]]... -o OUTPUT.wav [--backend BACKEND]",
        description: "Chorus is a recipe alias for chorus modulation. It lowers to the same typed effect pipeline as `render --fx 'chorus ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("--wave WAVE", "Default modulation wave: sine or triangle."),
            (
                "--stage STAGE",
                "Chorus stage as delay, decay, speed, and depth; repeat for multiple stages.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "flanger",
        summary: "add flanger modulation",
        synopsis: "auralis flanger INPUT.wav [--delay MS] [--depth MS] [--regen PERCENT] [--width PERCENT] [--speed HZ] [--wave WAVE] [--phase PERCENT] [--interpolation MODE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Flanger is a recipe alias for swept-delay flanger modulation. It lowers to the same typed effect pipeline as `render --fx 'flanger ...'`.",
        options: &[
            ("--delay MS", "Base delay in milliseconds."),
            ("--depth MS", "Sweep depth in milliseconds."),
            ("--regen PERCENT", "Regeneration percentage."),
            ("--width PERCENT", "Wet width percentage."),
            ("--speed HZ", "Modulation speed."),
            ("--wave WAVE", "Modulation wave: sine or triangle."),
            ("--phase PERCENT", "Stereo phase percentage."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "phaser",
        summary: "add phaser modulation",
        synopsis: "auralis phaser INPUT.wav [--gain-in GAIN] [--gain-out GAIN] [--delay MS] [--regen AMOUNT] [--speed HZ] [--wave WAVE] [--interpolation MODE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Phaser is a recipe alias for swept-delay phaser modulation. It lowers to the same typed effect pipeline as `render --fx 'phaser ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            ("--delay MS", "Delay in milliseconds."),
            ("--regen AMOUNT", "Regeneration amount."),
            ("--speed HZ", "Modulation speed."),
            ("--wave WAVE", "Modulation wave: sine or triangle."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "oops",
        summary: "extract out-of-phase stereo",
        synopsis: "auralis oops INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Oops is a recipe alias for extracting out-of-phase stereo content. It lowers to the same typed effect pipeline as `render --fx oops`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "riaa",
        summary: "apply RIAA equalization",
        synopsis: "auralis riaa INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Riaa is a recipe alias for vinyl playback equalization. It lowers to the same typed effect pipeline as `render --fx riaa`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "swap",
        summary: "swap adjacent channel pairs",
        synopsis: "auralis swap INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Swap is a recipe alias for exchanging adjacent channel pairs. It lowers to the same typed effect pipeline as `render --fx swap`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "contrast",
        summary: "enhance sample contrast",
        synopsis: "auralis contrast INPUT.wav [--amount AMOUNT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Contrast is a recipe alias for phase contrast enhancement. It lowers to the same typed effect pipeline as `render --fx 'contrast ...'`.",
        options: &[
            ("--amount AMOUNT", "Contrast amount from 0 to 100."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "overdrive",
        summary: "apply overdrive distortion",
        synopsis: "auralis overdrive INPUT.wav [--gain GAIN] [--color COLOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Overdrive is a recipe alias for applying one overdrive distortion stage. It lowers to the same typed effect pipeline as `render --fx 'overdrive ...'`.",
        options: &[
            ("--gain GAIN", "Overdrive gain."),
            ("--color COLOR", "Overdrive color."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "saturation",
        summary: "apply saturation distortion",
        synopsis: "auralis saturation INPUT.wav [--type TYPE] [--blend BLEND] [--offset OFFSET] [--parameter VALUE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Saturation is a recipe alias for applying one saturation curve. It lowers to the same typed effect pipeline as `render --fx 'saturation ...'`.",
        options: &[
            (
                "--type TYPE",
                "Saturation curve type: tanh, sqrt, or diode.",
            ),
            ("--blend BLEND", "Wet/dry blend amount."),
            ("--offset OFFSET", "Input offset before saturation."),
            (
                "--parameter VALUE",
                "Curve-specific parameter: drive, color, or threshold.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "dcshift",
        summary: "shift DC level",
        synopsis: "auralis dcshift INPUT.wav SHIFT [--limiter-gain GAIN] -o OUTPUT.wav [--backend BACKEND]",
        description: "Dcshift is a recipe alias for shifting sample DC offset. It lowers to the same typed effect pipeline as `render --fx 'dcshift ...'`.",
        options: &[
            ("SHIFT", "DC shift amount."),
            ("--limiter-gain GAIN", "Optional limiter gain."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "vol",
        summary: "apply SoX-ng volume scaling",
        synopsis: "auralis vol INPUT.wav GAIN [--type TYPE] [--limiter-gain GAIN] -o OUTPUT.wav [--backend BACKEND]",
        description: "Vol is a recipe alias for SoX-ng volume scaling. It lowers to the same typed effect pipeline as `render --fx 'vol ...'`.",
        options: &[
            ("GAIN", "Volume gain value, for example `0.5` or `-6dB`."),
            (
                "--type TYPE",
                "Gain interpretation: amplitude, power, or dB.",
            ),
            ("--limiter-gain GAIN", "Optional limiter gain."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "softvol",
        summary: "apply soft volume changes",
        synopsis: "auralis softvol INPUT.wav [--volume VOLUME] [--double-time SECONDS] [--headroom DB] -o OUTPUT.wav [--backend BACKEND]",
        description: "Softvol is a recipe alias for soft volume adjustment. It lowers to the same typed effect pipeline as `render --fx 'softvol ...'`.",
        options: &[
            ("--volume VOLUME", "Volume multiplier."),
            (
                "--double-time SECONDS",
                "Seconds required for volume doubling.",
            ),
            ("--headroom DB", "Extra headroom in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "tremolo",
        summary: "apply tremolo modulation",
        synopsis: "auralis tremolo INPUT.wav SPEED_HZ [--depth PERCENT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Tremolo is a recipe alias for amplitude modulation. It lowers to the same typed effect pipeline as `render --fx 'tremolo ...'`.",
        options: &[
            ("SPEED_HZ", "Modulation speed in Hz."),
            ("--depth PERCENT", "Modulation depth percentage."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "speed",
        summary: "change playback speed",
        synopsis: "auralis speed INPUT.wav FACTOR -o OUTPUT.wav [--backend BACKEND]",
        description: "Speed is a recipe alias for changing playback speed and output sample rate. It lowers to the same typed effect pipeline as `render --fx 'speed ...'`.",
        options: &[
            ("FACTOR", "Speed factor, or cents with a `c` suffix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "tempo",
        summary: "change tempo without changing pitch",
        synopsis: "auralis tempo INPUT.wav FACTOR [--quick] [--profile PROFILE] [--segment MS [--search MS [--overlap MS]]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Tempo is a recipe alias for time stretching without pitch shift. It lowers to the same typed effect pipeline as `render --fx 'tempo ...'`.",
        options: &[
            ("FACTOR", "Tempo factor."),
            ("--quick", "Prefer quicker search."),
            (
                "--profile PROFILE",
                "Tuning profile: music, speech, or linear.",
            ),
            ("--segment MS", "Segment length in milliseconds."),
            ("--search MS", "Search length in milliseconds."),
            ("--overlap MS", "Overlap length in milliseconds."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "pitch",
        summary: "shift pitch without changing tempo",
        synopsis: "auralis pitch INPUT.wav CENTS [--quick] [--segment MS [--search MS [--overlap MS]]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Pitch is a recipe alias for shifting pitch while preserving duration. It lowers to the same typed effect pipeline as `render --fx 'pitch ...'`.",
        options: &[
            ("CENTS", "Pitch shift in cents."),
            ("--quick", "Prefer quicker search."),
            ("--segment MS", "Segment length in milliseconds."),
            ("--search MS", "Search length in milliseconds."),
            ("--overlap MS", "Overlap length in milliseconds."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bass",
        summary: "boost or cut bass frequencies",
        synopsis: "auralis bass INPUT.wav DB [--frequency HZ] [--width WIDTH] -o OUTPUT.wav [--backend BACKEND]",
        description: "Bass is a recipe alias for one low-shelf EQ stage. It lowers to the same typed effect pipeline as `render --fx 'bass ...'`.",
        options: &[
            ("DB", "Shelf gain in dB."),
            ("--frequency HZ", "Shelf frequency in Hz."),
            (
                "--width WIDTH",
                "Shelf width, accepting values like `0.5s`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "treble",
        summary: "boost or cut treble frequencies",
        synopsis: "auralis treble INPUT.wav DB [--frequency HZ] [--width WIDTH] -o OUTPUT.wav [--backend BACKEND]",
        description: "Treble is a recipe alias for one high-shelf EQ stage. It lowers to the same typed effect pipeline as `render --fx 'treble ...'`.",
        options: &[
            ("DB", "Shelf gain in dB."),
            ("--frequency HZ", "Shelf frequency in Hz."),
            (
                "--width WIDTH",
                "Shelf width, accepting values like `0.5s`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "equalizer",
        summary: "apply one peaking equalizer band",
        synopsis: "auralis equalizer INPUT.wav --frequency HZ --width WIDTH --gain DB -o OUTPUT.wav [--backend BACKEND]",
        description: "Equalizer is a recipe alias for one peaking EQ band. It lowers to the same typed effect pipeline as `render --fx 'equalizer ...'`.",
        options: &[
            ("--frequency HZ", "Center frequency in Hz."),
            (
                "--width WIDTH",
                "Band width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--gain DB", "Band gain in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "allpass",
        summary: "apply an all-pass filter",
        synopsis: "auralis allpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Allpass is a recipe alias for one all-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'allpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole simple form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "band",
        summary: "apply a resonator band-pass filter",
        synopsis: "auralis band INPUT.wav --frequency HZ [--width WIDTH] [--unpitched] -o OUTPUT.wav [--backend BACKEND]",
        description: "Band is a recipe alias for one resonator band-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'band ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--unpitched", "Use the unpitched noise mode."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bandpass",
        summary: "apply an RBJ band-pass filter",
        synopsis: "auralis bandpass INPUT.wav --frequency HZ --width WIDTH [--constant-skirt] -o OUTPUT.wav [--backend BACKEND]",
        description: "Bandpass is a recipe alias for one RBJ band-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'bandpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--constant-skirt", "Use constant-skirt-gain mode."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bandreject",
        summary: "apply an RBJ band-reject filter",
        synopsis: "auralis bandreject INPUT.wav --frequency HZ --width WIDTH -o OUTPUT.wav [--backend BACKEND]",
        description: "Bandreject is a recipe alias for one RBJ band-reject filter stage. It lowers to the same typed effect pipeline as `render --fx 'bandreject ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "highpass",
        summary: "apply a high-pass filter",
        synopsis: "auralis highpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Highpass is a recipe alias for one high-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'highpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter cutoff frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "lowpass",
        summary: "apply a low-pass filter",
        synopsis: "auralis lowpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Lowpass is a recipe alias for one low-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'lowpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter cutoff frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "fade",
        summary: "fade one audio file in or out",
        synopsis: "auralis fade INPUT.wav [--in FRAMES] [--out FRAMES] [--curve CURVE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Fade is a recipe alias for applying one fade envelope. It lowers to the same typed effect pipeline as `render --fx 'fade ...'`.",
        options: &[
            (
                "--in FRAMES",
                "Fade-in length in frames, accepting values like `24000` or `24000f`.",
            ),
            (
                "--out FRAMES",
                "Fade-out length in frames, accepting values like `24000` or `24000f`.",
            ),
            (
                "--curve CURVE",
                "Fade curve family: linear, quarter-sine, half-sine, log, or parabola.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "delay",
        summary: "delay audio channels",
        synopsis: "auralis delay INPUT.wav --position POSITION... -o OUTPUT.wav [--backend BACKEND]",
        description: "Delay is a recipe alias for per-channel delay positions. It lowers to the same typed effect pipeline as `render --fx 'delay ...'`.",
        options: &[
            (
                "--position POSITION",
                "Delay position such as `2s`, `0.25`, or `+1s`; repeat per channel.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "pad",
        summary: "add silence padding",
        synopsis: "auralis pad INPUT.wav [--start FRAMES] [--at FRAMES@POSITION]... [--end FRAMES] -o OUTPUT.wav [--backend BACKEND]",
        description: "Pad is a recipe alias for inserting silence before, after, or inside one audio file. It lowers to the same typed effect pipeline as `render --fx 'pad ...'`.",
        options: &[
            ("--start FRAMES", "Silence to prepend, in frames."),
            ("--at FRAMES@POSITION", "Positioned silence insert."),
            ("--end FRAMES", "Silence to append, in frames."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "repeat",
        summary: "append finite copies",
        synopsis: "auralis repeat INPUT.wav [COUNT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Repeat is a recipe alias for appending finite copies of one audio file. It lowers to the same typed effect pipeline as `render --fx 'repeat ...'`.",
        options: &[
            ("COUNT", "Number of extra copies to append."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "downsample",
        summary: "keep every Nth sample",
        synopsis: "auralis downsample INPUT.wav [FACTOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Downsample is a recipe alias for dropping samples by an integer factor. It lowers to the same typed effect pipeline as `render --fx 'downsample ...'`.",
        options: &[
            ("FACTOR", "Integer downsample factor."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "upsample",
        summary: "insert zero samples",
        synopsis: "auralis upsample INPUT.wav [FACTOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Upsample is a recipe alias for inserting zero samples between input samples. It lowers to the same typed effect pipeline as `render --fx 'upsample ...'`.",
        options: &[
            ("FACTOR", "Integer upsample factor."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "hilbert",
        summary: "apply Hilbert transform",
        synopsis: "auralis hilbert INPUT.wav [--taps TAPS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Hilbert is a recipe alias for phase shifting with a Hilbert transform FIR. It lowers to the same typed effect pipeline as `render --fx 'hilbert ...'`.",
        options: &[
            ("--taps TAPS", "Optional odd FIR tap count."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "loudness",
        summary: "apply loudness compensation",
        synopsis: "auralis loudness INPUT.wav [--gain DB] [--reference DB] [--half-points N] -o OUTPUT.wav [--backend BACKEND]",
        description: "Loudness is a recipe alias for loudness compensation filtering. It lowers to the same typed effect pipeline as `render --fx 'loudness ...'`.",
        options: &[
            ("--gain DB", "Gain in dB."),
            ("--reference DB", "Reference level in dB."),
            ("--half-points N", "Number of FIR half-points."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "dither",
        summary: "apply deterministic dithering",
        synopsis: "auralis dither INPUT.wav [--sloped] [--noise-shape shibata] [--precision BITS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Dither is a recipe alias for deterministic dither processing. It lowers to the same typed effect pipeline as `render --fx 'dither ...'`.",
        options: &[
            ("--sloped", "Use sloped TPDF dither."),
            ("--noise-shape SHAPE", "Noise-shaping filter: shibata."),
            ("--precision BITS", "Target precision in bits."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "reverb",
        summary: "apply stereo reverberation",
        synopsis: "auralis reverb INPUT.wav [--wet-only] [--reverberance PERCENT] [--hf-damping PERCENT] [--room-scale PERCENT] [--stereo-depth PERCENT] [--pre-delay MS] [--wet-gain DB] -o OUTPUT.wav [--backend BACKEND]",
        description: "Reverb is a recipe alias for stereo reverberation. It lowers to the same typed effect pipeline as `render --fx 'reverb ...'`.",
        options: &[
            ("--wet-only", "Output only the wet reverberated signal."),
            ("--reverberance PERCENT", "Reverberance amount."),
            ("--hf-damping PERCENT", "High-frequency damping amount."),
            ("--room-scale PERCENT", "Room scale amount."),
            ("--stereo-depth PERCENT", "Stereo depth amount."),
            ("--pre-delay MS", "Pre-delay in milliseconds."),
            ("--wet-gain DB", "Wet gain in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "stretch",
        summary: "change duration with windowed stretching",
        synopsis: "auralis stretch INPUT.wav [FACTOR] [--window MS] [--fade SHAPE] [--shift RATIO [--fading RATIO]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Stretch is a recipe alias for basic windowed cross-fade stretching. It lowers to the same typed effect pipeline as `render --fx 'stretch ...'`.",
        options: &[
            ("FACTOR", "Stretch factor."),
            ("--window MS", "Analysis window length in milliseconds."),
            (
                "--fade SHAPE",
                "Fade shape: linear, sqrt, half, or quarter.",
            ),
            ("--shift RATIO", "Window shift ratio."),
            ("--fading RATIO", "Cross-fade ratio."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "mix",
        summary: "mix audio files",
        synopsis: "auralis mix INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Mix is a recipe alias for combining two or more inputs. It lowers to the same typed render pipeline as `render --combine mix --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to mix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "concat",
        summary: "concatenate audio files",
        synopsis: "auralis concat INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Concat is a recipe alias for joining two or more inputs end-to-end. It lowers to the same typed render pipeline as `render --combine concatenate --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to concatenate."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "mix-power",
        summary: "equal-power mix audio files",
        synopsis: "auralis mix-power INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Mix-power is a recipe alias for combining two or more inputs with equal-power scaling. It lowers to the same typed render pipeline as `render --combine mix-power --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to equal-power mix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "merge",
        summary: "merge audio channels",
        synopsis: "auralis merge INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Merge is a recipe alias for placing every input channel into one multichannel output. It lowers to the same typed render pipeline as `render --combine merge --input ...`.",
        options: &[
            (
                "INPUT",
                "Two or more WAV input files whose channels should be merged.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "multiply",
        summary: "multiply audio files",
        synopsis: "auralis multiply INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Multiply is a recipe alias for multiplying corresponding samples from two or more inputs. It lowers to the same typed render pipeline as `render --combine multiply --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to multiply."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "render",
        summary: "run one ordered DSP pipeline",
        synopsis: "auralis render INPUT.wav -o OUTPUT.wav [--fx EFFECT]... [--chain CHAIN] [options]",
        description: "Render is the primary linear-chain entry point. It combines optional additional inputs, parses typed effect syntax, applies output-boundary policies, and writes one output artifact.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--fx EFFECT", "Append one ordered typed effect string."),
            (
                "--chain CHAIN",
                "Use a compact pipe-delimited effect chain.",
            ),
            (
                "--combine METHOD",
                "Combine multiple inputs before effects.",
            ),
            ("--backend BACKEND", "Request scalar or simd processing."),
            ("--channels CHANNELS", "Set the output channel count."),
            ("--rate RATE", "Set the output sample rate."),
            ("--guard", "Apply clip guard at the output boundary."),
            (
                "--norm [DB]",
                "Normalize the output boundary to a peak target.",
            ),
            (
                "--dither",
                "Apply deterministic TPDF dither before PCM16 encoding.",
            ),
        ],
    },
    ManPage {
        name: "pipe",
        summary: "run one compact DSP expression",
        synopsis: "auralis pipe INPUT.wav EXPR -o OUTPUT.wav [--backend BACKEND]",
        description: "Pipe is a compact exploratory form for one ordered stream. It parses the expression as the same pipe-delimited typed effect chain accepted by `render --chain`.",
        options: &[
            ("EXPR", "Pipe-delimited ordered effect expression."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "plan",
        summary: "preview graph execution",
        synopsis: "auralis plan SPEC [--target TARGET] [--json] [--locked]",
        description: "Plan validates an Auralis graph spec, exposes streaming segments, whole-buffer barriers, fanout points, and output targets, and can emit a machine-readable JSON form for tooling.",
        options: &[
            ("--target TARGET", "Plan only the named target."),
            ("--json", "Emit machine-readable JSON output."),
            (
                "--locked",
                "Require a matching Auralis.lock before planning.",
            ),
        ],
    },
    ManPage {
        name: "run",
        summary: "execute an Auralis graph spec",
        synopsis: "auralis run [SPEC] [--target TARGET] [--locked]",
        description: "Run executes the currently supported source-to-chain-to-sink subset of graph specs. Unsupported graph nodes should be inspected with `plan` first.",
        options: &[
            ("--target TARGET", "Run only the named target."),
            (
                "--locked",
                "Require a matching Auralis.lock before running.",
            ),
        ],
    },
    ManPage {
        name: "init",
        summary: "create a graph spec scaffold",
        synopsis: "auralis init [SPEC]",
        description: "Init creates a minimal Auralis graph spec with one source, one empty chain, and one sink. It uses create-new semantics and will not overwrite an existing file.",
        options: &[(
            "SPEC",
            "Graph spec path to create, defaulting to Auralis.toml.",
        )],
    },
    ManPage {
        name: "cache",
        summary: "inspect or clear local persistent cache state",
        synopsis: "auralis cache status [--root DIR] [--json]\nauralis cache clear [--root DIR] --yes",
        description: "Cache status reports whether the local persistent cache directory exists and summarizes file count and bytes. Cache clear removes files under the selected cache root after explicit confirmation.",
        options: &[
            ("status", "Print cache directory status."),
            ("clear", "Remove files under the cache root."),
            (
                "--root DIR",
                "Cache root to inspect or clear, defaulting to .auralis/cache.",
            ),
            ("--json", "Emit machine-readable JSON output."),
            ("--yes", "Confirm cache clear removal."),
        ],
    },
    ManPage {
        name: "check",
        summary: "validate effect syntax or graph specs",
        synopsis: "auralis check [SPEC] [--locked] [--fx EFFECT]... [--chain CHAIN] [--effects-file FILE]",
        description: "Check validates either one graph spec or one effect-input mode. For graph specs, the default mode refreshes Auralis.lock while `--locked` requires an up-to-date lock.",
        options: &[
            (
                "--locked",
                "Require a matching Auralis.lock instead of refreshing it.",
            ),
            ("--fx EFFECT", "Validate one typed effect string."),
            ("--chain CHAIN", "Validate a compact pipe-delimited chain."),
            (
                "--effects-file FILE",
                "Validate a SoX-ng-style effects file.",
            ),
        ],
    },
    ManPage {
        name: "ops",
        summary: "inspect the typed operation registry",
        synopsis: "auralis ops [EFFECT] [--schema json]",
        description: "Ops lists implemented typed operations, resolves aliases, and can emit machine-readable registry metadata for tooling.",
        options: &[("--schema json", "Emit machine-readable JSON output.")],
    },
];

pub fn print_man_page(topic: Option<&str>) -> Result<(), CliError> {
    let page = topic
        .map(|topic| {
            MAN_PAGES
                .iter()
                .find(|page| page.name == topic)
                .ok_or_else(|| CliError::UnknownManTopic {
                    topic: topic.to_owned(),
                })
        })
        .transpose()?
        .unwrap_or(&MAN_PAGES[0]);

    println!("NAME");
    println!("  {} - {}", page.name, page.summary);
    println!();
    println!("SYNOPSIS");
    println!("  {}", page.synopsis);
    println!();
    println!("DESCRIPTION");
    println!("  {}", page.description);
    if !page.options.is_empty() {
        println!();
        println!("OPTIONS");
        for (name, description) in page.options {
            println!("  {name}");
            println!("    {description}");
        }
    }

    Ok(())
}
