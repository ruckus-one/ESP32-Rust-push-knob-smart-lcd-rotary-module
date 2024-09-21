use esp_idf_hal::gpio::{InputPin, Level, OutputPin};
use esp_idf_svc::{hal::gpio::{PinDriver, Pull}, timer::EspTimer};
use std::{thread::{self, JoinHandle}, time::Duration};


use esp_idf_svc::timer::EspTimerService;
use std::sync::Arc;

use std::sync::{mpsc::channel, Mutex};

pub struct Rotencoder<T1: InputPin + OutputPin, T2: InputPin + OutputPin> {
    clk: T1,
    dt: T2,
    cb: Arc<Mutex<dyn FnMut(i8) + Send + 'static>>,
}

impl<T1: InputPin + OutputPin, T2: InputPin + OutputPin> Rotencoder<T1, T2> {
    pub fn with_callback(clk: T1, dt: T2, cb: Arc<Mutex<dyn FnMut(i8) + Send + 'static>>) -> Self {
        Self { clk, dt, cb }
    }

    pub fn start_thread(self) -> JoinHandle<EspTimer<'static>> {
        let timer_service = EspTimerService::new().unwrap();

        return thread::Builder::new()
            .stack_size(2000)
            .spawn(move || {
                let mut button_1 = PinDriver::input(self.clk).unwrap();
                button_1.set_pull(Pull::Up).unwrap();
                let mut button_2 = PinDriver::input(self.dt).unwrap();
                button_2.set_pull(Pull::Up).unwrap();

                let (tx, rx) = channel::<i8>();

                let callback_timer = {
                    let mut prev = 0;
                    let mut internal_counter: i8 = 0;

                    timer_service.timer(move || {
                        let a = button_1.get_level();
                        let b = button_2.get_level();
                        let curr = Rotencoder::<T1, T2>::graycode_to_binary(a, b);
                        let diff = prev - curr;

                        if diff == -1 || diff == 3 {
                            internal_counter -= 1;
                            prev = curr;
                        } else if diff == 1 || diff == -3 {
                            internal_counter += 1;
                            prev = curr;
                        }

                        if internal_counter >= 4 {
                            tx.send(1).unwrap();
                            internal_counter = 0;
                        } else if internal_counter <= -4 {
                            tx.send(-1).unwrap();
                            internal_counter = 0;
                        }
                    })
                    .unwrap()
                };
                callback_timer.every(Duration::from_micros(244)).unwrap();
                
                loop {

                    match rx.try_recv() {
                        Ok(state) => {
                            match self.cb.try_lock() {
                                Ok(mut cb) => {
                                    cb(state);
                                }
                                Err(_) => (),
                            }
                        }
                        Err(_) => (),
                    }
    
                    std::thread::sleep(Duration::from_millis(20));
                }

            })
            .unwrap()
    }

    fn graycode_to_binary(a: Level, b: Level) -> i8 {
        if a == Level::Low && b == Level::Low {
            return 0
        } else if a == Level::Low && b == Level::High {
            return 1
        } else if a == Level::High && b == Level::High {
            return 2
        }
    
        return 3
    }
}
