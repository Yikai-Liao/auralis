use auralis_core::Decibels;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_decibels,
    reject_extra_arguments, render_f64,
};
use crate::{Gain, GainChannelMode, GainHeadroom};

pub(super) fn parse_gain(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut options = GainOptionFlags::default();
    let mut value_args = args;

    while let Some((option, rest)) = value_args.split_first() {
        if !is_option_like(option) {
            break;
        }
        parse_gain_options(effect, option, &mut options)?;
        value_args = rest;
    }
    validate_gain_option_combination(effect, options)?;

    let db = match value_args {
        [] => Decibels::new(0.0).map_err(|source| EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "gain-dB",
            source,
        })?,
        [db] => parse_decibels(effect, "gain-dB", db)?,
        [db, rest @ ..] => {
            reject_extra_arguments(effect, rest)?;
            parse_decibels(effect, "gain-dB", db)?
        }
    };

    let gain = match (
        options.contains(GainOptionFlags::RECLAIM_HEADROOM),
        options.contains(GainOptionFlags::RESERVE_HEADROOM),
    ) {
        (false, false) => Gain::new(db),
        (false, true) => Gain::reserve_headroom(db),
        (true, false) => Gain::reclaim_headroom(db),
        (true, true) => Gain::reclaim_and_reserve_headroom(db),
    }
    .with_normalize_if(options.contains(GainOptionFlags::NORMALIZE))
    .with_limiter_if(options.contains(GainOptionFlags::LIMITER))
    .with_channel_mode(channel_mode(options));

    Ok(EffectCommand::Gain(gain))
}

pub(super) fn render_gain(gain: Gain) -> Vec<String> {
    let mut tokens = vec!["gain".to_owned()];
    match gain.channel_mode {
        GainChannelMode::None => {}
        GainChannelMode::Equalize => tokens.push("-e".to_owned()),
        GainChannelMode::Balance => tokens.push("-B".to_owned()),
        GainChannelMode::BalanceNoClip => tokens.push("-b".to_owned()),
    }
    if gain.normalize {
        tokens.push("-n".to_owned());
    }
    match gain.headroom {
        GainHeadroom::None => {}
        GainHeadroom::Reserve => tokens.push("-h".to_owned()),
        GainHeadroom::Reclaim => tokens.push("-r".to_owned()),
        GainHeadroom::ReclaimAndReserve => tokens.push("-rh".to_owned()),
    }
    if gain.limiter {
        tokens.push("-l".to_owned());
    }
    tokens.push(render_f64(gain.db.as_f64()));
    tokens
}

#[derive(Debug, Default, Clone, Copy)]
struct GainOptionFlags {
    bits: u8,
}

impl GainOptionFlags {
    const RESERVE_HEADROOM: u8 = 1 << 0;
    const RECLAIM_HEADROOM: u8 = 1 << 1;
    const NORMALIZE: u8 = 1 << 2;
    const LIMITER: u8 = 1 << 3;
    const EQUALIZE: u8 = 1 << 4;
    const BALANCE: u8 = 1 << 5;
    const BALANCE_NO_CLIP: u8 = 1 << 6;

    const fn contains(self, flag: u8) -> bool {
        self.bits & flag != 0
    }

    const fn insert(&mut self, flag: u8) {
        self.bits |= flag;
    }

    const fn channel_mode_count(self) -> u8 {
        self.contains(Self::EQUALIZE) as u8
            + self.contains(Self::BALANCE) as u8
            + self.contains(Self::BALANCE_NO_CLIP) as u8
            + self.contains(Self::RECLAIM_HEADROOM) as u8
    }
}

fn parse_gain_options(
    effect: &'static str,
    option: &str,
    flags: &mut GainOptionFlags,
) -> CommandResult<()> {
    let mut chars = option.strip_prefix('-').unwrap_or_default().chars();
    let Some(first) = chars.next() else {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_owned(),
        });
    };

    for option_char in std::iter::once(first).chain(chars) {
        match option_char {
            'h' => flags.insert(GainOptionFlags::RESERVE_HEADROOM),
            'r' => flags.insert(GainOptionFlags::RECLAIM_HEADROOM),
            'n' => flags.insert(GainOptionFlags::NORMALIZE),
            'l' => flags.insert(GainOptionFlags::LIMITER),
            'e' => flags.insert(GainOptionFlags::EQUALIZE),
            'B' => flags.insert(GainOptionFlags::BALANCE),
            'b' => flags.insert(GainOptionFlags::BALANCE_NO_CLIP),
            unsupported => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: format!("-{unsupported}"),
                });
            }
        }
    }

    Ok(())
}

fn validate_gain_option_combination(
    effect: &'static str,
    flags: GainOptionFlags,
) -> CommandResult<()> {
    if flags.channel_mode_count() > 1 {
        return Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "only one of -e, -B, -b and -r may be given",
        });
    }
    if flags.contains(GainOptionFlags::NORMALIZE)
        && flags.contains(GainOptionFlags::RECLAIM_HEADROOM)
    {
        return Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "only one of -n and -r may be given",
        });
    }
    if flags.contains(GainOptionFlags::LIMITER) && flags.contains(GainOptionFlags::RESERVE_HEADROOM)
    {
        return Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "only one of -l and -h may be given",
        });
    }
    Ok(())
}

const fn channel_mode(options: GainOptionFlags) -> GainChannelMode {
    if options.contains(GainOptionFlags::EQUALIZE) {
        GainChannelMode::Equalize
    } else if options.contains(GainOptionFlags::BALANCE) {
        GainChannelMode::Balance
    } else if options.contains(GainOptionFlags::BALANCE_NO_CLIP) {
        GainChannelMode::BalanceNoClip
    } else {
        GainChannelMode::None
    }
}
