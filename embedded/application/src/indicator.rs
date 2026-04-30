use std::{
    sync::{Arc, mpsc},
    thread,
};

use anyhow::Result;
use esp_idf_hal::{
    delay::FreeRtos,
    gpio::OutputPin,
    ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, SpeedMode, config::TimerConfig},
    units::KiloHertz,
};

use crate::{
    app::{App, IndicatorEvent},
    util::ForceLock,
};

enum Mode {
    Idle,
    Active,
}

pub fn init<C, T, S, P>(app: Arc<App>, channel: C, timer: T, pin: P) -> Result<()>
where
    C: LedcChannel<SpeedMode = S> + 'static,
    T: LedcTimer<SpeedMode = S> + 'static,
    S: SpeedMode,
    P: OutputPin + 'static,
{
    let timer_driver =
        LedcTimerDriver::new(timer, &TimerConfig::new().frequency(KiloHertz(25).into()))?;
    let mut driver = LedcDriver::new(channel, &timer_driver, pin)?;
    let max = driver.get_max_duty();

    let (tx, rx) = mpsc::sync_channel(4);
    *app.indicator.force_lock() = Some(tx);

    thread::spawn(move || {
        let mut mode = Mode::Idle;
        loop {
            match mode {
                // Blink while NMEA bus is idle
                Mode::Idle => {
                    driver.fade_with_time(max, 500, true).unwrap();
                    driver.fade_with_time(0, 500, true).unwrap();
                    FreeRtos::delay_ms(1000);
                }
                // Solid once frames are received
                Mode::Active => {
                    driver.fade_with_time(max, 500, true).unwrap();
                    FreeRtos::delay_ms(1000);
                }
            }

            if let Ok(event) = rx.try_recv() {
                match event {
                    IndicatorEvent::CanOnline if matches!(mode, Mode::Idle) => {
                        mode = Mode::Active;
                    }
                    _ => {}
                }
            }
        }
    });

    Ok(())
}
