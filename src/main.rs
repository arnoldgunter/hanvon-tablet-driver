use evdev::{
    AbsInfo, AbsoluteAxisType, AttributeSet, EventType, InputEvent, Key, UinputAbsSetup,
    uinput::VirtualDeviceBuilder,
};
use futures_lite::stream;
use nusb::{
    DeviceInfo, MaybeFuture,
    hotplug::HotplugEvent,
    transfer::{Buffer, In, Interrupt},
};
use std::thread;
use std::time::Duration;

const CONTACT_THRESHOLD: u8 = 56;
const RELEASE_THRESHOLD: u8 = 51;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watch = nusb::watch_devices().unwrap();
    let hanvon_vendor_id = 0x0B57;

    for device in nusb::list_devices().wait()? {
        if device.vendor_id() == hanvon_vendor_id {
            start_tablet_stream(device);
        }
    }

    for event in stream::block_on(watch) {
        match event {
            HotplugEvent::Connected(d) => {
                if d.vendor_id() == hanvon_vendor_id {
                    start_tablet_stream(d);
                }
            }
            HotplugEvent::Disconnected(_) => {}
        }
    }
    Ok(())
}

fn start_tablet_stream(info: DeviceInfo) {
    println!(
        "Hanvon tablet detected (USB {:04x}:{:04x}). Starting input stream...",
        info.vendor_id(),
        info.product_id()
    );

    match info.open().wait() {
        Ok(device) => {
            thread::spawn(move || {
                if let Err(e) = read_tablet_stream(device) {
                    eprintln!("Tablet input stream stopped with an error: {e}");
                }
            });
        }
        Err(e) => eprintln!(
            "Could not open the tablet. Check the USB connection and udev permissions: {e}"
        ),
    }
}

// Read tablet reports and forward them to the virtual input device.
fn read_tablet_stream(device: nusb::Device) -> Result<(), Box<dyn std::error::Error>> {
    let configuration = device.active_configuration().map_err(|error| {
        format!("Tablet has no active USB configuration; check the USB connection: {error}")
    })?;
    if configuration.configuration_value() != 1 {
        return Err(format!(
            "Tablet selected unexpected USB configuration {}",
            configuration.configuration_value()
        )
        .into());
    }

    // The tablet has no kernel driver bound, so claim its configured HID interface.
    let interface = device.claim_interface(0).wait()?;

    // Endpoint 0x81 is the tablet's interrupt IN endpoint.
    let endpoint_address = 0x81;
    let mut endpoint = interface.endpoint::<Interrupt, In>(endpoint_address)?;
    let mut mouse = create_virtual_tablet()?;
    let mut pen_down = false;
    let mut pen_button_down = false;
    let mut filtered_position = None;

    println!("Tablet driver is active. Waiting for pen input...");
    endpoint.submit(Buffer::new(10));

    loop {
        match endpoint.wait_next_complete(Duration::MAX) {
            Some(completion) => match completion.into_result() {
                Ok(data) => {
                    if data.len() >= 10 && data[0] == 2 {
                        let position = decode_position(&data);
                        let (x, y) = smooth_position(&mut filtered_position, position);
                        let current_pen_down = if pen_down {
                            data[6] > RELEASE_THRESHOLD
                        } else {
                            data[6] > CONTACT_THRESHOLD
                        };
                        let pressure = if current_pen_down {
                            decode_pressure(&data)
                        } else {
                            0
                        };
                        let mut events = vec![
                            InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, x),
                            InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, y),
                            InputEvent::new(
                                EventType::ABSOLUTE,
                                AbsoluteAxisType::ABS_PRESSURE.0,
                                pressure,
                            ),
                        ];
                        if current_pen_down != pen_down {
                            events.push(InputEvent::new(
                                EventType::KEY,
                                Key::BTN_LEFT.code(),
                                i32::from(current_pen_down),
                            ));
                            pen_down = current_pen_down;
                        }
                        let current_pen_button_down = data[1] & 0x02 != 0;
                        if current_pen_button_down != pen_button_down {
                            events.push(InputEvent::new(
                                EventType::KEY,
                                Key::BTN_RIGHT.code(),
                                i32::from(current_pen_button_down),
                            ));
                            pen_button_down = current_pen_button_down;
                        }
                        mouse.emit(&events)?;
                    }
                    endpoint.submit(data);
                }
                Err(e) => {
                    println!("Tablet disconnected or USB read failed: {e}");
                    break;
                }
            },
            None => {
                println!("Tablet input stream ended unexpectedly.");
                break;
            }
        }
    }
    Ok(())
}

fn create_virtual_tablet() -> Result<evdev::uinput::VirtualDevice, Box<dyn std::error::Error>> {
    let mut buttons = AttributeSet::<Key>::new();
    buttons.insert(Key::BTN_LEFT);
    buttons.insert(Key::BTN_RIGHT);

    let abs_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_X, AbsInfo::new(0, 0, 0x27de, 0, 0, 0));
    let abs_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, AbsInfo::new(0, 0, 0x1cfe, 0, 0, 0));

    Ok(VirtualDeviceBuilder::new()?
        .name("Hanvon Tablet Mouse")
        .with_absolute_axis(&abs_x)?
        .with_absolute_axis(&abs_y)?
        .with_absolute_axis(&UinputAbsSetup::new(
            AbsoluteAxisType::ABS_PRESSURE,
            AbsInfo::new(0, 0, 1023, 0, 0, 0),
        ))?
        .with_keys(&buttons)?
        .build()?)
}

fn decode_position(report: &[u8]) -> (i32, i32) {
    let x = i32::from(u16::from_be_bytes([report[2], report[3]]));
    let y = i32::from(u16::from_be_bytes([report[4], report[5]]));
    (x, y)
}

fn decode_pressure(report: &[u8]) -> i32 {
    i32::from(u16::from_be_bytes([report[6], report[7]]) >> 6)
}

fn smooth_position(previous: &mut Option<(i32, i32)>, current: (i32, i32)) -> (i32, i32) {
    let filtered = match *previous {
        Some((previous_x, previous_y)) => (
            (previous_x * 3 + current.0) / 4,
            (previous_y * 3 + current.1) / 4,
        ),
        None => current,
    };
    *previous = Some(filtered);
    filtered
}
