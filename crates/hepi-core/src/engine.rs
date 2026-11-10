//! Â§4 â€” khuÃ´n engine. Má»i engine MUST Ä‘i qua `Guarded`.
//!
//! `Guarded` giá»¯ ba báº¥t biáº¿n mÃ  báº£n thÃ¢n engine khÃ´ng tá»± giá»¯ ná»•i:
//!   R4 idempotent â€” start Ã—2 â‰¡ Ã—1, stop khi chÆ°a start lÃ  no-op.
//!   R5 timeout â€” start vÆ°á»£t `remote_thread_timeout_ms` â‡’ stop + E_TIMEOUT.
//!   Teardown â€” `Drop` gá»i `stop`, nÃªn Ä‘Æ°á»ng thoÃ¡t sá»›m vÃ  panic váº«n Ä‘Ã³ng handle.
//!
//! Timeout á»Ÿ Ä‘Ã¢y lÃ  má»m: Ä‘o sau khi hÃ m tráº£ vá». Timeout cá»©ng náº±m trong engine
//! (vÃ­ dá»¥ `WaitForSingleObject` vá»›i `remote_thread_timeout_ms`).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::settings::{Settings, Tuning, TUNING};
use crate::{log, Res};

pub struct Config {
    pub settings: Settings,
    pub tuning: &'static Tuning,
}

impl Config {
    pub fn from_settings(settings: Settings) -> Self {
        Self { settings, tuning: &TUNING }
    }
}

pub trait Engine {
    type H;

    fn start(&mut self, cfg: &Config) -> Res<Self::H>;
    fn verify(&self, h: &Self::H) -> bool;
    fn patch(&mut self, h: &mut Self::H) -> Res<()>;
    fn rollback(&mut self, h: &mut Self::H) -> Res<()>;
    fn stop(&mut self, h: Self::H);
}

static LIVE: AtomicUsize = AtomicUsize::new(0);

pub fn live_handles() -> usize {
    LIVE.load(Ordering::SeqCst)
}

/// Â§9.7 â€” khÃ´ng cÃ²n handle nÃ o sau teardown.
pub fn assert_no_live_handles() -> bool {
    live_handles() == 0
}

enum State<H> {
    Idle,
    Running(H),
    Stopped,
}

pub struct Guarded<E: Engine> {
    engine: E,
    label: &'static str,
    state: State<E::H>,
}

impl<E: Engine> Guarded<E> {
    pub fn new(engine: E, label: &'static str) -> Self {
        Self { engine, label, state: State::Idle }
    }

    pub fn start(&mut self, cfg: &Config) -> Res<()> {
        match self.state {
            State::Running(_) => return Ok(()),
            State::Stopped => self.state = State::Idle,
            State::Idle => {}
        }
        let t0 = Instant::now();
        let h = match self.engine.start(cfg) {
            Ok(h) => h,
            Err(e) => {
                log::err(self.label, e, "start failed");
                return Err(e);
            }
        };
        let spent = t0.elapsed().as_millis() as u32;
        if spent > cfg.tuning.remote_thread_timeout_ms {
            log::kv(self.label, "start over budget", "ms", &spent.to_string());
            self.engine.stop(h);
            return Err(crate::E::Timeout);
        }
        LIVE.fetch_add(1, Ordering::SeqCst);
        self.state = State::Running(h);
        log::kv(self.label, "started", "ms", &spent.to_string());
        Ok(())
    }

    pub fn verify(&self) -> bool {
        match &self.state {
            State::Running(h) => self.engine.verify(h),
            _ => false,
        }
    }

    /// Lá»—i â‡’ rollback táº¡i chá»—, khÃ´ng Ä‘á»ƒ handle ná»­a vá»i.
    pub fn patch(&mut self) -> Res<()> {
        let State::Running(h) = &mut self.state else {
            return Err(crate::E::State);
        };
        match self.engine.patch(h) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::err(self.label, e, "patch failed, rolling back");
                let _ = self.engine.rollback(h);
                Err(e)
            }
        }
    }

    pub fn rollback(&mut self) -> Res<()> {
        match &mut self.state {
            State::Running(h) => self.engine.rollback(h),
            _ => Ok(()),
        }
    }

    pub fn stop(&mut self) {
        let prev = std::mem::replace(&mut self.state, State::Stopped);
        if let State::Running(mut h) = prev {
            if let Err(e) = self.engine.rollback(&mut h) {
                log::err(self.label, e, "rollback failed during stop");
            }
            self.engine.stop(h);
            LIVE.fetch_sub(1, Ordering::SeqCst);
            log::line(self.label, "stopped");
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }
}

impl<E: Engine> Drop for Guarded<E> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::E;

    struct Mock {
        stops: u32,
        patches: u32,
        rolls: u32,
        fail_patch: bool,
    }

    impl Engine for Mock {
        type H = u32;

        fn start(&mut self, _cfg: &Config) -> Res<u32> {
            Ok(1)
        }
        fn verify(&self, h: &u32) -> bool {
            *h == 1
        }
        fn patch(&mut self, _h: &mut u32) -> Res<()> {
            self.patches += 1;
            if self.fail_patch {
                Err(E::Access)
            } else {
                Ok(())
            }
        }
        fn rollback(&mut self, _h: &mut u32) -> Res<()> {
            self.rolls += 1;
            Ok(())
        }
        fn stop(&mut self, _h: u32) {
            self.stops += 1;
        }
    }

    fn mock(fail: bool) -> Mock {
        Mock { stops: 0, patches: 0, rolls: 0, fail_patch: fail }
    }

    #[test]
    fn start_is_idempotent() {
        let base = live_handles();
        let mut g = Guarded::new(mock(false), "T");
        let cfg = Config::from_settings(Settings::default());
        assert!(g.start(&cfg).is_ok());
        assert!(g.start(&cfg).is_ok());
        assert_eq!(live_handles(), base + 1);
        assert!(g.verify());
        g.stop();
        assert_eq!(live_handles(), base);
    }

    #[test]
    fn patch_error_rolls_back() {
        let mut g = Guarded::new(mock(true), "T");
        let cfg = Config::from_settings(Settings::default());
        g.start(&cfg).unwrap();
        assert_eq!(g.patch(), Err(E::Access));
        assert_eq!(g.engine.rolls, 1);
        g.stop();
    }

    #[test]
    fn stop_twice_and_drop_are_safe() {
        let base = live_handles();
        {
            let mut g = Guarded::new(mock(false), "T");
            let cfg = Config::from_settings(Settings::default());
            g.start(&cfg).unwrap();
            g.stop();
            g.stop();
            assert_eq!(g.engine.stops, 1);
        }
        assert_eq!(live_handles(), base);
        assert!(assert_no_live_handles() || live_handles() == base);
    }

    #[test]
    fn patch_before_start_is_state_error() {
        let mut g = Guarded::new(mock(false), "T");
        assert_eq!(g.patch(), Err(E::State));
        assert!(!g.verify());
    }
}

