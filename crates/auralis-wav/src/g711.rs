use crate::{Result, WavError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum G711Kind {
    ULaw,
    ALaw,
}

impl G711Kind {
    pub(crate) const fn format_tag(self) -> u16 {
        match self {
            Self::ALaw => 0x0006,
            Self::ULaw => 0x0007,
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ULaw => "u-law",
            Self::ALaw => "A-law",
        }
    }

    pub(crate) fn decode_byte(self, sample: u8) -> f32 {
        let linear = match self {
            Self::ULaw => ulaw_to_i16(sample),
            Self::ALaw => alaw_to_i16(sample),
        };
        f32::from(linear) / 32768.0
    }

    pub(crate) fn encode_sample(
        self,
        sample: f32,
        sample_index: usize,
        channels: usize,
    ) -> Result<u8> {
        if !sample.is_finite() {
            return Err(WavError::NonFiniteSample {
                channel_index: sample_index % channels,
                frame_index: sample_index / channels,
            });
        }

        Ok(match self {
            Self::ULaw => i16_to_ulaw(quantize_for_g711(sample, 14)),
            Self::ALaw => i16_to_alaw(quantize_for_g711(sample, 13)),
        })
    }
}

fn quantize_for_g711(sample: f32, bits: u16) -> i16 {
    let scale = f32::from(1_u16 << (bits - 1));
    let min = -i32::from(1_u16 << (bits - 1));
    let max = i32::from((1_u16 << (bits - 1)) - 1);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "The finite sample is clipped and rounded into the target G.711 input range immediately before the cast."
    )]
    let quantized = (sample.clamp(-1.0, 1.0) * scale).round() as i32;
    let quantized = quantized.clamp(min, max);

    i16::try_from(quantized).expect("G.711 quantized sample fits in i16")
}

fn alaw_to_i16(sample: u8) -> i16 {
    let sample = sample ^ 0x55;
    let mut value = i16::from(sample & 0x0f) << 4;
    let segment = (sample & 0x70) >> 4;
    match segment {
        0 => value += 8,
        1 => value += 0x108,
        _ => {
            value += 0x108;
            value <<= i16::from(segment - 1);
        }
    }

    if sample & 0x80 != 0 { value } else { -value }
}

fn ulaw_to_i16(sample: u8) -> i16 {
    let sample = !sample;
    let mut value = (i16::from(sample & 0x0f) << 3) + 0x84;
    value <<= i16::from((sample & 0x70) >> 4);

    if sample & 0x80 != 0 {
        0x84 - value
    } else {
        value - 0x84
    }
}

fn i16_to_alaw(mut sample: i16) -> u8 {
    let mask = if sample >= 0 {
        0xd5
    } else {
        sample = -sample - 1;
        0x55
    };

    let segment = search_segment(
        i32::from(sample),
        &[0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff],
    );
    if segment >= 8 {
        return 0x7f ^ mask;
    }

    let mut encoded = u8::try_from(segment << 4).expect("A-law segment fits in u8");
    if segment < 2 {
        encoded |= u8::try_from((sample >> 1) & 0x0f).expect("A-law quantization fits in u8");
    } else {
        encoded |= u8::try_from((sample >> segment) & 0x0f).expect("A-law quantization fits in u8");
    }

    encoded ^ mask
}

fn i16_to_ulaw(mut sample: i16) -> u8 {
    let mask = if sample < 0 {
        sample = -sample;
        0x7f
    } else {
        0xff
    };
    sample = sample.min(8159) + (0x84 >> 2);

    let segment = search_segment(
        i32::from(sample),
        &[0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff],
    );
    if segment >= 8 {
        return 0x7f ^ mask;
    }

    let segment_shift = u32::try_from(segment + 1).expect("u-law segment shift fits in u32");
    let quantized = (i32::from(sample) >> segment_shift) & 0x0f;
    let encoded = u8::try_from((segment << 4) | usize::try_from(quantized).unwrap())
        .expect("u-law code fits in u8");
    encoded ^ mask
}

fn search_segment(value: i32, ends: &[i32; 8]) -> usize {
    ends.iter()
        .position(|&end| value <= end)
        .unwrap_or(ends.len())
}
