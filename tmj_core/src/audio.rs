use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Debug},
    hash::Hash,
    time::Duration,
};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub enum FadeCurve {
    Linear,
    Exponential,
    EaseInOut,
}

impl FadeCurve {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            FadeCurve::Linear => t,
            FadeCurve::Exponential => 1.0 - (-5.0 * t).exp(),
            FadeCurve::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackConfig {
    pub base_volume: f32,
    pub looped: bool,
    pub max_concurrent: usize,
    pub default_fade_duration: Duration,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            base_volume: 1.0,
            looped: false,
            max_concurrent: 2,
            default_fade_duration: Duration::from_millis(1500),
        }
    }
}

pub type AudioSource = Box<dyn Source<Item = i16> + Send>;

pub enum AudioOp {
    FadeIn {
        source: AudioSource,
        duration: Duration,
        curve: FadeCurve,
    },
    FadeOut {
        duration: Duration,
        curve: FadeCurve,
    },
    Play {
        source: AudioSource,
        volume: f32,
    },
    Wait(Duration),
    Stop,
}

impl fmt::Debug for AudioOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FadeIn {
                source: _source,
                duration,
                curve: _curve,
            } => f
                .debug_struct("FadeIn")
                .field("duration", duration)
                .finish(),
            Self::FadeOut { duration, curve } => f
                .debug_struct("FadeOut")
                .field("duration", duration)
                .field("curve", curve)
                .finish(),
            Self::Play { source: _source, volume } => f
                .debug_struct("Play")
                .field("volume", volume)
                .finish(),
            Self::Wait(arg0) => f.debug_tuple("Wait").field(arg0).finish(),
            Self::Stop => write!(f, "Stop"),
        }
    }
}

impl AudioOp {
    pub fn wait(duration: Duration) -> Self {
        Self::Wait(duration)
    }
    /// 快捷：淡入
    pub fn fade_in(source: AudioSource, duration: Duration) -> Self {
        Self::FadeIn {
            source,
            duration,
            curve: FadeCurve::Linear,
        }
    }

    /// 快捷：淡出
    pub fn fade_out(duration: Duration) -> Self {
        Self::FadeOut {
            duration,
            curve: FadeCurve::Linear,
        }
    }

    /// 快捷：播放
    pub fn play(source: AudioSource, volume: f32) -> Self {
        Self::Play { source, volume }
    }

    /// 设置曲线
    pub fn with_curve(mut self, curve: FadeCurve) -> Self {
        match &mut self {
            Self::FadeIn { curve: c, .. } => *c = curve,
            Self::FadeOut { curve: c, .. } => *c = curve,
            _ => {}
        }
        self
    }
}

pub struct AudioTrack {
    pub name: String,
    pub config: TrackConfig,
    pub volume_multiplier: f32,
    sinks: Vec<ManagedSink>,
    op_queue: VecDeque<AudioOp>,
    current_op_elapsed: Duration,
    waiting: bool,
}

impl AudioTrack {
    fn new(name: String, config: TrackConfig) -> Self {
        Self {
            name,
            config,
            volume_multiplier: 1.0,
            sinks: Vec::new(),
            op_queue: VecDeque::new(),
            current_op_elapsed: Duration::ZERO,
            waiting: false,
        }
    }

    fn create_sink(&mut self, stream_handle: &OutputStreamHandle, source: AudioSource) {
        if let Ok(sink) = Sink::try_new(stream_handle) {
            if self.config.looped {
                let source = source.repeat_infinite();
                sink.append(source);
            } else {
                sink.append(source);
            }

            let base_vol = self.config.base_volume * self.volume_multiplier;
            self.sinks.push(ManagedSink::new(sink, base_vol));
        }
    }

    fn get_primary_sink_mut(&mut self) -> Option<&mut ManagedSink> {
        self.sinks.iter_mut().filter(|s| !s.is_dead()).last()
    }

    pub fn queue_batch(&mut self, ops: Vec<AudioOp>) {
        for op in ops {
            self.op_queue.push_back(op);
        }
        info!("queue add, now: {:?}", self.op_queue);
    }

    pub fn queue(&mut self, op: AudioOp) {
        self.op_queue.push_back(op);
    }

    pub fn stop(&mut self) {
        for sink in &self.sinks {
            sink.sink.stop();
        }
        self.sinks.clear();
        self.op_queue.clear();
        self.waiting = false;
    }

    fn update(&mut self, stream_handle: &OutputStreamHandle, dt: Duration) {
        self.sinks.retain_mut(|sink| sink.update(dt));

        self.sinks.retain(|s| !s.is_dead());

        if self.waiting {
            self.current_op_elapsed += dt;
            if let Some(AudioOp::Wait(duration)) = self.op_queue.front() {
                if self.current_op_elapsed >= *duration {
                    self.op_queue.pop_front();
                    self.waiting = false;
                    self.current_op_elapsed = Duration::ZERO;
                }
            } else {
                self.waiting = false;
            }
            return;
        }

        while let Some(op) = self.op_queue.pop_front() {
            match op {
                AudioOp::FadeIn {
                    source,
                    duration,
                    curve,
                } => {
                    self.create_sink(stream_handle, source);
                    if let Some(new_sink) = self.sinks.last_mut() {
                        new_sink.start_fade(1.0, duration, curve);
                    }
                }
                AudioOp::FadeOut { duration, curve } => {
                    info!("track is try fade out {:?}", duration);
                    for sink in self.sinks.iter_mut().filter(|s| !s.is_dead()) {

                        info!("sink fade out {:?}", duration);
                        sink.start_fade(0.0, duration, curve);
                    }
                }
                AudioOp::Play { source, volume } => {
                    self.create_sink(stream_handle, source);
                    if let Some(sink) = self.sinks.last_mut() {
                        sink.current_volume = volume;
                        sink.target_volume = volume;
                        sink.apply_volume();
                    }
                }
                AudioOp::Wait(duration) => {
                    self.current_op_elapsed = Duration::ZERO;
                    self.waiting = true;
                    self.op_queue.push_front(AudioOp::Wait(duration));
                    break;
                }
                AudioOp::Stop => {
                    self.stop();
                    break;
                }
            }
        }
    }
}

struct ManagedSink {
    sink: Sink,
    base_volume: f32,
    current_volume: f32,
    fade_start_volume: f32,
    target_volume: f32,
    fade_progress: f32,
    fade_duration: Duration,
    is_fading: bool,
    curve: FadeCurve,
}

impl ManagedSink {
    fn new(sink: Sink, base_volume: f32) -> Self {
        Self {
            sink,
            base_volume,
            current_volume: 0.0,
            fade_start_volume: 0.0,
            target_volume: 0.0,
            fade_progress: 1.0,
            fade_duration: Duration::ZERO,
            is_fading: false,
            curve: FadeCurve::Linear,
        }
    }

    fn update(&mut self, dt: Duration) -> bool {
        if !self.is_fading || self.fade_duration.is_zero() {
            return true;
        }

        self.fade_progress += dt.as_secs_f32() / self.fade_duration.as_secs_f32();
        self.fade_progress = self.fade_progress.min(1.0);

        let t = self.curve.apply(self.fade_progress);
        self.current_volume =
            self.fade_start_volume + (self.target_volume - self.fade_start_volume) * t;
        info!("sink is update : volume {}", self.current_volume);
        self.apply_volume();

        if self.fade_progress >= 1.0 {
            self.is_fading = false;
            if self.target_volume == 0.0 {
                return false;
            }
        }
        true
    }

    fn apply_volume(&self) {
        let final_vol = self.base_volume * self.current_volume;
        self.sink.set_volume(final_vol);
    }

    fn start_fade(&mut self, target_volume: f32, duration: Duration, curve: FadeCurve) {
        self.target_volume = target_volume;
        self.fade_start_volume = self.current_volume;
        self.fade_progress = 0.0;
        self.fade_duration = duration;
        self.curve = curve;
        self.is_fading = true;
    }

    fn is_dead(&self) -> bool {
        self.sink.empty()
    }
}

pub struct AudioManager<K>
where
    K: Eq + Hash + Clone + Debug,
{
    stream_handle: OutputStreamHandle,
    _stream: OutputStream,
    tracks: std::collections::HashMap<K, AudioTrack>,
    master_volume: f32,
}

impl<K> AudioManager<K>
where
    K: Eq + Hash + Clone + Debug,
{
    pub fn new() -> Result<Self, rodio::StreamError> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        Ok(Self {
            stream_handle,
            _stream,
            tracks: HashMap::new(),
            master_volume: 1.0,
        })
    }

    pub fn create_track(&mut self, key: K, name: impl Into<String>, config: TrackConfig) {
        let track = AudioTrack::new(name.into(), config);
        self.tracks.insert(key, track);
    }

    pub fn track_mut(&mut self, key: &K) -> Option<&mut AudioTrack> {
        self.tracks.get_mut(key)
    }
    pub fn track(&self, key: &K) -> Option<&AudioTrack> {
        self.tracks.get(key)
    }

    pub fn transition(
        &mut self,
        from_key: &K,
        to_key: &K,
        source: AudioSource,
        duration: Duration,
        curve: FadeCurve,
    ) {
        if let Some(track) = self.tracks.get_mut(from_key) {
            track.queue(AudioOp::FadeOut { duration, curve });
        }
        if let Some(track) = self.tracks.get_mut(to_key) {
            track.queue(AudioOp::FadeIn {
                source,
                duration,
                curve,
            });
        }
    }

    pub fn update(&mut self, dt: Duration) {
        for track in self.tracks.values_mut() {
            track.update(&self.stream_handle, dt);
        }
    }
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn stop(&mut self, key: &K) {
        if let Some(track) = self.tracks.get_mut(key) {
            track.stop();
        }
    }

    pub fn stop_all(&mut self) {
        for track in self.tracks.values_mut() {
            track.stop();
        }
    }
}

impl<K> Default for AudioManager<K>
where
    K: Eq + Hash + Clone + Debug,
{
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio")
    }
}
