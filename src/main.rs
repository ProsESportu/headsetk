use core::str;
use std::{process::Command, time::Duration};

use ksni::{menu::StandardItem, Icon, Status};
use memoize::memoize;
use resvg::{
    tiny_skia::Pixmap,
    usvg::{Options, Transform},
};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct HeadsetControl {
    name: String,
    version: String,
    api_version: String,
    hidapi_version: String,
    device_count: i32,
    devices: Vec<Device>,
}
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Device {
    status: String,
    device: String,
    vendor: String,
    product: String,
    id_vendor: String,
    id_product: String,
    capabilities: Vec<String>,
    capabilities_str: Vec<String>,
    battery: Battery,
    chatmix: Option<i32>,
}
#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
struct Battery {
    status: BatteryStatus,
    level: i32,
}
#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
enum BatteryStatus {
    BATTERY_AVAILABLE,
    BATTERY_UNAVAILABLE,
    BATTERY_CHARGING,
}
struct MyTray<'a> {
    battery: Battery,
    opts: Options<'a>,
}
impl ksni::Tray for MyTray<'_> {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    // fn icon_name(&self) -> String {
    //     "battery".to_string()
    // }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let (argb, width, height) = render_icon(self.battery.clone(), &self.opts);
        vec![Icon {
            width: width as i32,
            height: height as i32,
            data: argb,
        }]
    }
    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "quit".to_string(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: self.battery.level.to_string(),
                ..Default::default()
            }
            .into(),
        ]
    }
    fn status(&self) -> ksni::Status {
        match self.battery.status {
            BatteryStatus::BATTERY_UNAVAILABLE => Status::Passive,
            _ => {
                if self.battery.level <= 20 {
                    Status::NeedsAttention
                } else {
                    Status::Active
                }
            }
        }
    }
    fn title(&self) -> String {
        "Battery Status of your headset".to_string()
    }
}
#[memoize(Ignore: opts,Capacity: 8)]
fn render_icon(battery: Battery, opts: &Options) -> (Vec<u8>, u32, u32) {
    let mut rgba = Pixmap::new(512, 512).unwrap();
    let tree = &resvg::usvg::Tree::from_str(
        include_str!("../ink.svg")
            .replace("BAT", battery.level.to_string().as_str())
            .replace(
                "FG_COLOR",
                match battery.status {
                    BatteryStatus::BATTERY_CHARGING => "00ff00",
                    BatteryStatus::BATTERY_UNAVAILABLE => "000000",
                    BatteryStatus::BATTERY_AVAILABLE => {
                        if battery.level <= 20 {
                            "ff0000"
                        } else {
                            "ffffff"
                        }
                    }
                },
            )
            .replace(
                "WIDTH",
                map_from_to(battery.level as f32, 0f32, 100f32, 71f32, 108f32)
                    .to_string()
                    .as_str(),
            )
            .as_str(),
        opts,
    )
    .unwrap();
    resvg::render(tree, Transform::identity(), &mut rgba.as_mut());
    let rgba_bytes = rgba.data();
    let argb = rgba_to_argb(rgba_bytes);
    (argb, rgba.width(), rgba.height())
}

fn rgba_to_argb(rgba_bytes: &[u8]) -> Vec<u8> {
    let mut argb = vec![0u8; rgba_bytes.len()];
    for (src, tgt) in Iterator::zip(rgba_bytes.chunks(4), argb.chunks_mut(4)) {
        tgt[0] = src[3];
        tgt[1] = src[0];
        tgt[2] = src[1];
        tgt[3] = src[2];
    }
    argb
}

fn get_battery(spawner: &mut Command) -> Battery {
    let process = spawner.output().unwrap();
    let stdout = process.stdout;
    parse_battery(stdout)
}
#[memoize(Capacity:8)]
fn parse_battery(stdout: Vec<u8>) -> Battery {
    let str = str::from_utf8(stdout.as_slice()).unwrap();
    let parsed: HeadsetControl = serde_json::from_str(str).unwrap();
    let battery = parsed.devices.first().unwrap().battery.clone();
    battery
}

fn map_from_to(
    input: f32,
    input_start: f32,
    input_end: f32,
    output_start: f32,
    output_end: f32,
) -> f32 {
    output_start + ((output_end - output_start) / (input_end - input_start)) * (input - input_start)
}
fn main() {
    println!("Hello, world!");
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();
    let mut child = Command::new("headsetcontrol");
    let spawner = child.arg("-o").arg("JSON");
    let service = ksni::TrayService::new(MyTray {
        battery: get_battery(spawner),
        opts: opt,
    });
    let handle = service.handle();
    service.spawn();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        handle.update(|tray: &mut MyTray| {
            tray.battery = get_battery(spawner);
        })
    }
}
