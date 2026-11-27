//! Sony DualSense gamepads on the Windows HID gamepad collection.

use std::ffi::CString;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice, HidResult};

use super::{
    A, B, DOWN, DUALSENSE_PADS, LEFT, LEFT_SHOULDER, LEFT_STICK, MENU, PadState, RIGHT,
    RIGHT_SHOULDER, RIGHT_STICK, UP, VIEW, X, Y,
};

const SONY: u16 = 0x054c;
const DUALSENSE: u16 = 0x0ce6;
const SCAN_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct Controllers {
    slots: [Option<Controller>; DUALSENSE_PADS],
    api: Option<HidApi>,
    scan: Option<JoinHandle<HidResult<HidApi>>>,
    next_scan: Option<Instant>,
}

impl Controllers {
    pub(super) fn poll(&mut self) {
        if let Some(scan) = self.scan.take_if(|scan| scan.is_finished())
            && let Ok(Ok(api)) = scan.join()
        {
            self.update_devices(&api);
            self.api = Some(api);
        }
        let now = Instant::now();
        if self.scan.is_none() && self.next_scan.is_none_or(|at| now >= at) {
            self.next_scan = Some(now + SCAN_INTERVAL);
            // Windows device enumeration can take longer than an input poll.
            let api = self.api.take();
            self.scan = std::thread::Builder::new()
                .name("DualSense discovery".into())
                .spawn(move || match api {
                    Some(mut api) => {
                        api.reset_devices()?;
                        api.add_devices(SONY, DUALSENSE)?;
                        Ok(api)
                    }
                    None => HidApi::new(),
                })
                .ok();
        }
        for slot in &mut self.slots {
            if let Some(pad) = slot
                && pad.poll().is_err()
            {
                *slot = None;
            }
        }
    }

    fn update_devices(&mut self, api: &HidApi) {
        let devices: Vec<_> = api
            .device_list()
            .filter(|info| {
                info.vendor_id() == SONY
                    && info.product_id() == DUALSENSE
                    && info.usage_page() == 0x01
                    && info.usage() == 0x05
            })
            .collect();
        for slot in &mut self.slots {
            if let Some(pad) = slot
                && !devices
                    .iter()
                    .any(|info| info.path() == pad.path.as_c_str())
            {
                *slot = None;
            }
        }
        for info in devices {
            if self
                .slots
                .iter()
                .flatten()
                .any(|pad| pad.path.as_c_str() == info.path())
            {
                continue;
            }
            let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) else {
                break;
            };
            if let Ok(device) = info.open_device(api) {
                tracing::info!(transport = ?info.bus_type(), "DualSense gamepad connected");
                *slot = Some(Controller {
                    path: info.path().to_owned(),
                    device,
                    state: None,
                    bluetooth: false,
                    motors: [0; 2],
                    sent_motors: Some([0; 2]),
                    retry_rumble: None,
                    sequence: 0,
                });
            }
        }
    }

    pub(super) fn state(&self, slot: usize) -> Option<PadState> {
        self.slots[slot].as_ref()?.state
    }

    pub(super) fn rumble(&mut self, slot: usize, low: u16, high: u16) {
        if let Some(pad) = &mut self.slots[slot] {
            pad.motors = [(low >> 8) as u8, (high >> 8) as u8];
        }
    }
}

struct Controller {
    path: CString,
    device: HidDevice,
    state: Option<PadState>,
    bluetooth: bool,
    motors: [u8; 2],
    sent_motors: Option<[u8; 2]>,
    retry_rumble: Option<Instant>,
    sequence: u8,
}

impl Controller {
    fn poll(&mut self) -> HidResult<()> {
        let mut report = [0; 78];
        // Drain old readings without waiting for another packet. A poll with
        // no new report must keep held buttons and analog positions intact.
        for _ in 0..64 {
            let len = self.device.read_timeout(&mut report, 0)?;
            if len == 0 {
                break;
            }
            if let Some(state) = decode(&report[..len]) {
                self.state = Some(state);
                self.bluetooth = len != 64;
            }
        }
        if self.state.is_some()
            && self.sent_motors != Some(self.motors)
            && self.retry_rumble.is_none_or(|at| Instant::now() >= at)
        {
            let mut output = [0; 78];
            let len = rumble_report(&mut output, self.bluetooth, self.sequence, self.motors);
            match self.device.write(&output[..len]) {
                Ok(written) if written == len => {
                    self.sent_motors = Some(self.motors);
                    self.retry_rumble = None;
                }
                result => {
                    // Keep reading input and retry the latest motor command, including a stop.
                    tracing::debug!(?result, "DualSense rumble write incomplete");
                    self.sent_motors = None;
                    self.retry_rumble = Some(Instant::now() + Duration::from_secs(1));
                }
            }
            self.sequence = self.sequence.wrapping_add(1);
        }
        Ok(())
    }
}

fn decode(report: &[u8]) -> Option<PadState> {
    // Windows pads the 10-byte simple Bluetooth report to 78 bytes.
    // Extended Bluetooth adds a tag before the common USB state layout.
    let (data, triggers, buttons) = match (report.first()?, report.len()) {
        (0x01, 10 | 78) => (&report[1..], 7, 4),
        (0x01, 64) => (&report[1..], 4, 7),
        (0x31, 78) => (&report[2..], 4, 7),
        _ => return None,
    };
    const HAT: [u32; 8] = [
        UP,
        UP | RIGHT,
        RIGHT,
        RIGHT | DOWN,
        DOWN,
        DOWN | LEFT,
        LEFT,
        LEFT | UP,
    ];
    const BUTTONS: [(u16, u32); 10] = [
        (0x0010, X),
        (0x0020, A),
        (0x0040, B),
        (0x0080, Y),
        (0x0100, LEFT_SHOULDER),
        (0x0200, RIGHT_SHOULDER),
        (0x1000, VIEW),
        (0x2000, MENU),
        (0x4000, LEFT_STICK),
        (0x8000, RIGHT_STICK),
    ];
    let hat = usize::from(data[buttons] & 0x0f);
    let bits = u16::from_le_bytes([data[buttons], data[buttons + 1]]);
    let mut pressed = HAT.get(hat).copied().unwrap_or(0);
    for (mask, button) in BUTTONS {
        if bits & mask != 0 {
            pressed |= button;
        }
    }
    let stick = |v: u8| (f32::from(v) - 128.0) / if v < 128 { 128.0 } else { 127.0 };
    Some(PadState {
        buttons: pressed,
        left_trigger: f32::from(data[triggers]) / 255.0,
        right_trigger: f32::from(data[triggers + 1]) / 255.0,
        left_x: stick(data[0]),
        left_y: -stick(data[1]),
        right_x: stick(data[2]),
        right_y: -stick(data[3]),
    })
}

fn rumble_report(
    report: &mut [u8; 78],
    bluetooth: bool,
    sequence: u8,
    [low, high]: [u8; 2],
) -> usize {
    let offset = if bluetooth {
        report[..3].copy_from_slice(&[0x31, sequence << 4, 0x10]);
        3
    } else {
        report[0] = 0x02;
        1
    };
    // Enable the two compatibility vibration motors. Other effects stay under their own flags.
    report[offset..offset + 4].copy_from_slice(&[0x03, 0, high, low]);
    if bluetooth {
        use zlib_rs::crc32::crc32;
        let crc = crc32(crc32(0, &[0xa2]), &report[..74]);
        report[74..].copy_from_slice(&crc.to_le_bytes());
        78
    } else {
        48
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_and_both_bluetooth_modes_give_the_same_reading() {
        let simple = [1, 0, 255, 128, 0, 0x23, 0x11, 0, 64, 255];
        let mut padded = [0; 78];
        padded[..simple.len()].copy_from_slice(&simple);
        let mut usb = [0; 64];
        usb[..10].copy_from_slice(&[1, 0, 255, 128, 0, 64, 255, 0x77, 0x23, 0x11]);
        let mut extended = [0; 78];
        extended[..2].copy_from_slice(&[0x31, 0x72]);
        extended[2..65].copy_from_slice(&usb[1..]);
        let expected = PadState {
            buttons: A | DOWN | RIGHT | LEFT_SHOULDER | VIEW,
            left_x: -1.0,
            left_y: -1.0,
            right_x: 0.0,
            right_y: 1.0,
            left_trigger: 64.0 / 255.0,
            right_trigger: 1.0,
        };
        for report in [simple.as_slice(), &padded, &usb, &extended] {
            assert_eq!(decode(report), Some(expected));
        }
        for report in [&[][..], &simple[..9], &extended[..64], &[0xff; 78]] {
            assert_eq!(decode(report), None);
        }
    }

    #[test]
    fn face_buttons_shoulders_and_stick_clicks_use_xbox_positions() {
        let mut simple = [1, 128, 128, 128, 128, 8, 0, 0, 0, 0];
        assert_eq!(decode(&simple), Some(PadState::default()));
        for (mask, expected) in [
            (0x10, X),
            (0x20, A),
            (0x40, B),
            (0x80, Y),
            (0x100, LEFT_SHOULDER),
            (0x200, RIGHT_SHOULDER),
            (0x1000, VIEW),
            (0x2000, MENU),
            (0x4000, LEFT_STICK),
            (0x8000, RIGHT_STICK),
        ] {
            simple[5..7].copy_from_slice(&(mask | 8u16).to_le_bytes());
            assert_eq!(decode(&simple).unwrap().buttons, expected);
        }
        for (hat, expected) in [
            UP,
            UP | RIGHT,
            RIGHT,
            RIGHT | DOWN,
            DOWN,
            DOWN | LEFT,
            LEFT,
            LEFT | UP,
            0,
        ]
        .into_iter()
        .enumerate()
        {
            simple[5..7].copy_from_slice(&[hat as u8, 0]);
            assert_eq!(decode(&simple).unwrap().buttons, expected);
        }
    }

    #[test]
    fn rumble_uses_the_transport_header_motor_order_and_bluetooth_crc() {
        let mut usb = [0; 78];
        assert_eq!(rumble_report(&mut usb, false, 3, [0x12, 0x34]), 48);
        assert_eq!(&usb[..5], &[2, 3, 0, 0x34, 0x12]);
        let mut bt = [0; 78];
        assert_eq!(rumble_report(&mut bt, true, 3, [0x12, 0x34]), 78);
        assert_eq!(&bt[..7], &[0x31, 0x30, 0x10, 3, 0, 0x34, 0x12]);
        assert_eq!(&bt[74..], &0x98cd22bau32.to_le_bytes());
    }
}
