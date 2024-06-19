use std::time::Duration;

/// Keeps track of time before next reconnection attempt
pub struct DoublingTimer {
    // Configuration
    flat: u32, // Number of attempts before doubling duration
    init_dur: Duration,

    // State
    cur_dur: Duration,
    rem: u32, // Remaining attempts before doubling
}

impl DoublingTimer {
    /// Constructs a new ReconnectionTimer in the reset state
    /// flat = 0 -> never double
    pub fn new(flat: u32, init_dur: Duration) -> Self {
        Self {
            flat,
            init_dur,
            cur_dur: init_dur,
            rem: flat,
        }
    }

    /// Returns the next reconnection attempt delay
    pub fn next(&mut self) -> Duration {
        let res = self.cur_dur;

        // Update for next attempt
        if self.flat != 0 {
            // flat = 0 -> Never double
            self.rem -= 1;
            if self.rem == 0 {
                self.rem = self.flat;
                self.cur_dur *= 2; // Double duration
            }
        }

        res
    }

    /// Resets the timer to the initial duration
    pub fn reset(&mut self) {
        self.cur_dur = self.init_dur;
        self.rem = self.flat;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubling_timer_normal() {
        let mut timer = DoublingTimer::new(2, Duration::from_millis(1000));

        assert_eq!(timer.next(), Duration::from_millis(1000));
        assert_eq!(timer.next(), Duration::from_millis(1000));
        assert_eq!(timer.next(), Duration::from_millis(2000));
        assert_eq!(timer.next(), Duration::from_millis(2000));
        assert_eq!(timer.next(), Duration::from_millis(4000));
        assert_eq!(timer.next(), Duration::from_millis(4000));

        timer.reset();

        assert_eq!(timer.next(), Duration::from_millis(1000));
        assert_eq!(timer.next(), Duration::from_millis(1000));
        assert_eq!(timer.next(), Duration::from_millis(2000));
        assert_eq!(timer.next(), Duration::from_millis(2000));
        assert_eq!(timer.next(), Duration::from_millis(4000));
        assert_eq!(timer.next(), Duration::from_millis(4000));
    }

    #[test]
    fn doubling_timer_nodouble() {
        let mut timer = DoublingTimer::new(0, Duration::from_millis(2500));

        assert_eq!(timer.next(), Duration::from_millis(2500));
        assert_eq!(timer.next(), Duration::from_millis(2500));
        assert_eq!(timer.next(), Duration::from_millis(2500));
        assert_eq!(timer.next(), Duration::from_millis(2500));
        assert_eq!(timer.next(), Duration::from_millis(2500));
        assert_eq!(timer.next(), Duration::from_millis(2500));

        timer.reset();

        assert_eq!(timer.next(), Duration::from_millis(2500));
    }
}
