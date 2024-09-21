use esp_idf_hal::
    gpio::{InputPin, InterruptType, OutputPin}
;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use std::{
    sync::{mpsc::channel, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration
};

pub struct Button<T: InputPin + OutputPin> {
    pin: T,
    cb: Arc<Mutex<dyn FnMut(ButtonState) + Send + 'static>>,
}

pub enum ButtonState {
    Pressed,
    Released,
}

impl<T: InputPin + OutputPin> Button<T> {
    pub fn new(pin: T, cb: Arc<Mutex<dyn FnMut(ButtonState) + Send + 'static>>) -> Self {
        Self { pin, cb }
    }

    pub fn spawn_thread(self) -> JoinHandle<()> {
        thread::Builder::new().stack_size(2000).spawn(move || {
            let mut btn = PinDriver::input(self.pin).unwrap();
            btn.set_pull(Pull::Up).unwrap();
            btn.set_interrupt_type(InterruptType::AnyEdge).unwrap();

            let (tx, rx) = channel::<ButtonState>();

            let mut pulse_counter = 0_u8;
            unsafe {
                btn.subscribe(move || {
                    pulse_counter = pulse_counter + 1;

                    match pulse_counter {
                        1 => {
                            tx.send(ButtonState::Pressed).unwrap();
                        }
                        _ => {
                            tx.send(ButtonState::Released).unwrap();
                        }
                    }

                    if pulse_counter > 1 {
                        pulse_counter = 0;
                    }
                })
                .unwrap()
            }

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

                btn.enable_interrupt().unwrap();
                std::thread::sleep(Duration::from_millis(20));
            }
        }).unwrap()
    }
}