use std::time::Instant;

pub struct TimeProvider {
    last_update: Instant,
    cached_hour: u32,
    cached_minute: u32,
    cached_second: u32,
    cached_ampm: &'static str,
}

impl Default for TimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider {
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            cached_hour: 0,
            cached_minute: 0,
            cached_second: 0,
            cached_ampm: "AM",
        }
    }

    pub fn update(&mut self) {
        // Stub: in real impl, use chrono or libstd time
        // For now, use a simple counter
        let now = self.last_update.elapsed().as_secs();
        self.cached_second = (now % 60) as u32;
        self.cached_minute = ((now / 60) % 60) as u32;
        self.cached_hour = ((now / 3600) % 12) as u32;
        self.cached_ampm = if now / 3600 >= 12 { "PM" } else { "AM" };
    }

    pub fn time_24h(&self) -> String {
        format!("{:02}:{:02}", self.cached_hour, self.cached_minute)
    }

    pub fn time_12h(&self) -> String {
        format!(
            "{:02}:{:02} {}",
            self.cached_hour, self.cached_minute, self.cached_ampm
        )
    }

    pub fn hour(&self) -> u32 {
        self.cached_hour
    }
    pub fn minute(&self) -> u32 {
        self.cached_minute
    }
    pub fn ampm(&self) -> &str {
        self.cached_ampm
    }
}

pub enum TrackState {
    Stopped,
    Playing { title: String, artist: String },
}

pub struct NowPlaying {
    state: TrackState,
}

impl Default for NowPlaying {
    fn default() -> Self {
        Self::new()
    }
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            state: TrackState::Stopped,
        }
    }

    pub fn update(&mut self, track: Option<(&str, &str)>) {
        // Stub: in real impl, query MPRIS
        match track {
            Some((title, artist)) => {
                self.state = TrackState::Playing {
                    title: title.to_string(),
                    artist: artist.to_string(),
                }
            }
            None => self.state = TrackState::Stopped,
        }
    }

    pub fn state(&self) -> &TrackState {
        &self.state
    }

    pub fn track_title(&self) -> Option<&str> {
        match &self.state {
            TrackState::Playing { title, .. } => Some(title),
            _ => None,
        }
    }

    pub fn track_artist(&self) -> Option<&str> {
        match &self.state {
            TrackState::Playing { artist, .. } => Some(artist),
            _ => None,
        }
    }
}

pub struct NowBar<'a> {
    pub time: &'a TimeProvider,
    pub track: &'a NowPlaying,
}

impl<'a> NowBar<'a> {
    pub fn new(time: &'a TimeProvider, track: &'a NowPlaying) -> Self {
        Self { time, track }
    }

    pub fn render_time(&self) -> String {
        match self.track.track_title() {
            Some(title) => format!("\u{266A} {}  |  {}", title, self.time.time_24h()),
            None => self.time.time_24h(),
        }
    }

    pub fn render_notification(&self) -> Option<String> {
        match self.track.track_title() {
            Some(title) => {
                let artist = self.track.track_artist().unwrap_or("Unknown");
                Some(format!("\u{266B} {} \u{2014} {}", title, artist))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_provider_format() {
        let tp = TimeProvider::new();
        assert_eq!(tp.time_24h().len(), 5); // "00:00"
    }

    #[test]
    fn now_playing_stopped() {
        let np = NowPlaying::new();
        assert!(np.track_title().is_none());
        assert!(np.track_artist().is_none());
    }

    #[test]
    fn now_bar_time_only() {
        let tp = TimeProvider::new();
        let np = NowPlaying::new();
        let bar = NowBar::new(&tp, &np);
        let rendered = bar.render_time();
        assert!(!rendered.contains("\u{266A}"));
    }

    #[test]
    fn now_bar_with_track() {
        let tp = TimeProvider::new();
        let mut np = NowPlaying::new();
        np.update(Some(("Song", "Artist")));
        let bar = NowBar::new(&tp, &np);
        let rendered = bar.render_time();
        assert!(rendered.contains("\u{266A}"));
    }

    #[test]
    fn notification_shows_track() {
        let tp = TimeProvider::new();
        let mut np = NowPlaying::new();
        np.update(Some(("Song", "Artist")));
        let bar = NowBar::new(&tp, &np);
        let notif = bar.render_notification();
        assert!(notif.is_some());
        assert!(notif.unwrap().contains("Song"));
    }
}
