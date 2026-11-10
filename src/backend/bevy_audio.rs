//! Implementation to integrate `bevy_procedural_audio` into `bevy_audio`.

use {
    super::Backend,
    crate::dsp_source::{DspSource, IterMono, SourceType},
    bevy::{
        audio::AddAudioSource,
        prelude::{App, AudioSource, Decodable},
    },
    std::time::Duration,
};

/// The backend for `bevy_audio`.
#[allow(clippy::module_name_repetitions)]
pub struct BevyAudioBackend;

impl Decodable for DspSource {
    type Decoder = DspDecoder;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn decoder(&self) -> Self::Decoder {
        let (remaining, total_duration) = match self.source_type {
            SourceType::Static { duration } => (
                Some((duration * self.sample_rate).round() as usize),
                Some(Duration::from_secs_f32(duration)),
            ),
            SourceType::Dynamic => (None, None),
        };

        DspDecoder {
            iter: self.clone().into_iter().into_mono(),
            remaining,
            total_duration,
        }
    }
}

/// The `rodio` source used to play a [`DspSource`] through `bevy_audio`.
///
/// Static sources end after their duration, which lets the sink run empty
/// so that `PlaybackSettings` such as `DESPAWN` and `REMOVE` take effect.
/// Dynamic sources never end.
pub struct DspDecoder {
    iter: IterMono,
    /// Samples left to play, or `None` if the source is dynamic.
    remaining: Option<usize>,
    total_duration: Option<Duration>,
}

impl Iterator for DspDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(remaining) = &mut self.remaining {
            if *remaining == 0 {
                return None;
            }
            *remaining -= 1;
        }

        self.iter.next()
    }
}

impl rodio::Source for DspDecoder {
    fn current_span_len(&self) -> Option<usize> {
        self.remaining
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(1).unwrap()
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(self.iter.0.sample_rate as u32).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}

impl Backend for BevyAudioBackend {
    type StaticAudioSource = AudioSource;

    fn init_app(app: &mut App) {
        app.add_audio_source::<DspSource>();
    }

    fn convert_to_audio_source(
        dsp_source: crate::dsp_source::DspSource,
    ) -> Self::StaticAudioSource {
        let bytes = dsp_source.to_bytes().into();

        AudioSource { bytes }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]

    use {
        super::*,
        crate::DEFAULT_SAMPLE_RATE,
        fundsp::prelude32::*,
        rodio::Source,
    };

    #[test]
    fn static_source_ends() {
        let source = DspSource::new(
            || constant(1.0),
            *DEFAULT_SAMPLE_RATE,
            SourceType::Static { duration: 0.5 },
        );

        let decoder = source.decoder();
        assert_eq!(decoder.total_duration(), Some(Duration::from_secs_f32(0.5)));

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let expected = (0.5 * *DEFAULT_SAMPLE_RATE).round() as usize;
        assert_eq!(decoder.count(), expected);
    }

    #[test]
    fn dynamic_source_does_not_end() {
        let source = DspSource::new(|| constant(1.0), *DEFAULT_SAMPLE_RATE, SourceType::Dynamic);

        let decoder = source.decoder();
        assert_eq!(decoder.total_duration(), None);
        assert_eq!(decoder.take(100_000).count(), 100_000);
    }
}

// fn play_queued_audio

// impl DspAudioExt for Audio<AudioSource> {
//     type Assets = Assets<AudioSource>;
//     type Settings = PlaybackSettings;
//     // type Sink = Handle<AudioSink>;

//     fn play_dsp_with_settings(
//         &mut self,
//         assets: &mut Self::Assets,
//         source: &DspSource,
//         settings: Self::Settings,
//     ) -> Self::Sink {
//         let audio = BevyAudioBackend::convert_to_audio_source(source.clone());
//         let handle = assets.add(audio);
//         self.play_with_settings(handle, settings)
//     }
// }

// impl DspAudioExt for DspSource {
//     type Assets = Assets<DspSource>;
//     type Settings = PlaybackSettings;
//     type Sink = AudioSink;

//     fn play_dsp_with_settings(
//         &mut self,
//         assets: &mut Self::Assets,
//         source: &DspSource,
//         settings: Self::Settings,
//     ) -> Self::Sink {
//         let handle = assets.add(source.clone());
//         self.play_with_settings(handle, settings)
//     }
// }
