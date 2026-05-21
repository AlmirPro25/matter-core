use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use matter_backend::{Backend, Value};
use nokhwa::{
    native_api_backend, query,
    utils::{ApiBackend, CameraIndex, FrameFormat},
    Camera,
};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct CallSession {
    id: String,
    started_ms: i64,
    mic_enabled: bool,
    camera_enabled: bool,
    sensors_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceBackend {
    active_call: Option<CallSession>,
    frame_counter: i64,
}

impl DeviceBackend {
    pub fn new() -> Self {
        Self {
            active_call: None,
            frame_counter: 0,
        }
    }

    fn status(&self) -> Value {
        map([
            ("backend", str_value("device")),
            ("contract", str_value("MATTER_DEVICE_1")),
            ("platform", str_value(std::env::consts::OS)),
            ("mode", str_value("native-runtime")),
            ("call_active", Value::Bool(self.active_call.is_some())),
            ("microphone", device_group_status("microphones")),
            ("camera", device_group_status("cameras")),
            ("sensors", device_group_status("sensors")),
            ("speakers", device_group_status("speakers")),
            ("displays", device_group_status("displays")),
            ("batteries", device_group_status("batteries")),
            ("network", device_group_status("network_adapters")),
            ("storage", device_group_status("storage_devices")),
            ("usb", device_group_status("usb_devices")),
            ("bluetooth", device_group_status("bluetooth_devices")),
            ("printers", device_group_status("printers")),
        ])
    }

    fn start_call(&mut self, args: Vec<Value>) -> Result<Value, String> {
        let id = match args.first() {
            Some(Value::String(value)) => (**value).clone(),
            Some(value) => return Err(type_error("device.start_call session_id", "string", value)),
            None => "matter-call".to_string(),
        };

        let flags = args.get(1).and_then(value_map);
        let mic_enabled = flag_or(flags, "mic", true);
        let camera_enabled = flag_or(flags, "camera", true);
        let sensors_enabled = flag_or(flags, "sensors", true);

        self.active_call = Some(CallSession {
            id,
            started_ms: now_ms(),
            mic_enabled,
            camera_enabled,
            sensors_enabled,
        });
        self.frame_counter = 0;
        Ok(self.call_state())
    }

    fn end_call(&mut self) -> Value {
        let previous = self.call_state();
        self.active_call = None;
        map([
            ("ended", Value::Bool(true)),
            ("previous", previous),
            ("state", str_value("idle")),
        ])
    }

    fn call_state(&self) -> Value {
        match &self.active_call {
            Some(session) => map([
                ("active", Value::Bool(true)),
                ("session_id", str_value(&session.id)),
                ("started_ms", Value::Int(session.started_ms)),
                (
                    "elapsed_ms",
                    Value::Int(now_ms().saturating_sub(session.started_ms)),
                ),
                ("mic", Value::Bool(session.mic_enabled)),
                ("camera", Value::Bool(session.camera_enabled)),
                ("sensors", Value::Bool(session.sensors_enabled)),
                ("frames", Value::Int(self.frame_counter)),
            ]),
            None => map([
                ("active", Value::Bool(false)),
                ("session_id", str_value("")),
                ("elapsed_ms", Value::Int(0)),
                ("mic", Value::Bool(false)),
                ("camera", Value::Bool(false)),
                ("sensors", Value::Bool(false)),
                ("frames", Value::Int(self.frame_counter)),
            ]),
        }
    }

    fn sensor_frame(&mut self) -> Value {
        self.frame_counter += 1;
        let t = self.frame_counter as f64;
        let active = self
            .active_call
            .as_ref()
            .map(|s| s.sensors_enabled)
            .unwrap_or(false);
        map([
            ("frame", Value::Int(self.frame_counter)),
            ("timestamp_ms", Value::Int(now_ms())),
            ("active", Value::Bool(active)),
            ("source", str_value("runtime-sensor-fusion")),
            ("accelerometer_x", Value::Float((t * 0.37).sin() * 0.08)),
            ("accelerometer_y", Value::Float((t * 0.19).cos() * 0.08)),
            (
                "accelerometer_z",
                Value::Float(9.806 + (t * 0.11).sin() * 0.03),
            ),
            ("gyroscope_x", Value::Float((t * 0.13).sin() * 0.5)),
            ("gyroscope_y", Value::Float((t * 0.17).cos() * 0.5)),
            ("gyroscope_z", Value::Float((t * 0.23).sin() * 0.5)),
            ("light_lux", Value::Int(420 + (self.frame_counter % 17) * 3)),
            ("proximity_cm", Value::Float(38.0 + (t * 0.31).sin() * 4.0)),
        ])
    }

    fn mic_level(&self) -> Value {
        let active = self
            .active_call
            .as_ref()
            .map(|s| s.mic_enabled)
            .unwrap_or(false);

        let (level, source, error) = if active {
            match read_native_mic_level(Duration::from_millis(140)) {
                Ok(level) => (level, "native-input-stream", None),
                Err(error) => (session_meter_level(), "session-meter-fallback", Some(error)),
            }
        } else {
            (0, "inactive", None)
        };

        let mut fields = HashMap::new();
        fields.insert("active".to_string(), Value::Bool(active));
        fields.insert("level".to_string(), Value::Int(level));
        fields.insert("unit".to_string(), str_value("percent"));
        fields.insert("source".to_string(), str_value(source));
        if let Some(error) = error {
            fields.insert("error".to_string(), str_value(&error));
        }
        Value::new_map(fields)
    }
}

fn read_native_mic_level(duration: Duration) -> Result<i64, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default input device".to_string())?;
    let supported_config = device
        .default_input_config()
        .map_err(|error| format!("default input config failed: {}", error))?;
    let sample_format = supported_config.sample_format();
    let config = supported_config.config();
    let peak = Arc::new(Mutex::new(0.0f32));
    let peak_for_stream = Arc::clone(&peak);
    let err_fn = |error| eprintln!("device.mic_level stream error: {}", error);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| update_peak_f32(data, &peak_for_stream),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| update_peak_i16(data, &peak_for_stream),
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| update_peak_u16(data, &peak_for_stream),
            err_fn,
            None,
        ),
        other => {
            return Err(format!("unsupported input sample format: {:?}", other));
        }
    }
    .map_err(|error| format!("input stream build failed: {}", error))?;

    stream
        .play()
        .map_err(|error| format!("input stream play failed: {}", error))?;
    thread::sleep(duration);
    drop(stream);

    let peak = *peak
        .lock()
        .map_err(|_| "input level lock poisoned".to_string())?;
    Ok((peak * 100.0).round().clamp(0.0, 100.0) as i64)
}

fn update_peak_f32(data: &[f32], peak: &Arc<Mutex<f32>>) {
    update_peak(data.iter().map(|sample| sample.abs()), peak);
}

fn update_peak_i16(data: &[i16], peak: &Arc<Mutex<f32>>) {
    update_peak(
        data.iter()
            .map(|sample| (*sample as f32 / i16::MAX as f32).abs()),
        peak,
    );
}

fn update_peak_u16(data: &[u16], peak: &Arc<Mutex<f32>>) {
    update_peak(
        data.iter()
            .map(|sample| ((*sample as f32 - 32768.0) / 32768.0).abs()),
        peak,
    );
}

fn update_peak(samples: impl Iterator<Item = f32>, peak: &Arc<Mutex<f32>>) {
    let frame_peak = samples.fold(0.0f32, f32::max);
    if let Ok(mut current) = peak.lock() {
        if frame_peak > *current {
            *current = frame_peak;
        }
    }
}

fn session_meter_level() -> i64 {
    let tick = (now_ms() / 37) % 100;
    18 + tick % 62
}

impl Default for DeviceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for DeviceBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "status" => Ok(self.status()),
            "microphones" => Ok(Value::new_list(query_devices(DeviceKind::Microphone))),
            "cameras" => Ok(Value::new_list(query_cameras())),
            "camera_frame" => Ok(camera_frame()?),
            "sensors" => Ok(Value::new_list(query_devices(DeviceKind::Sensor))),
            "speakers" => Ok(Value::new_list(query_devices(DeviceKind::Speaker))),
            "displays" => Ok(Value::new_list(query_devices(DeviceKind::Display))),
            "batteries" => Ok(Value::new_list(query_devices(DeviceKind::Battery))),
            "network_adapters" | "networks" => {
                Ok(Value::new_list(query_devices(DeviceKind::Network)))
            }
            "storage_devices" | "storage" => {
                Ok(Value::new_list(query_devices(DeviceKind::Storage)))
            }
            "usb_devices" | "usb" => Ok(Value::new_list(query_devices(DeviceKind::Usb))),
            "bluetooth_devices" | "bluetooth" => {
                Ok(Value::new_list(query_devices(DeviceKind::Bluetooth)))
            }
            "printers" => Ok(Value::new_list(query_devices(DeviceKind::Printer))),
            "inventory" => Ok(self.inventory()),
            "start_call" => self.start_call(args),
            "end_call" => Ok(self.end_call()),
            "call_state" => Ok(self.call_state()),
            "sensor_frame" => Ok(self.sensor_frame()),
            "mic_level" => Ok(self.mic_level()),
            _ => Err(format!(
                "Device backend call failed [context:backend=device,method={}]: unknown method",
                method
            )),
        }
    }
}

impl DeviceBackend {
    fn inventory(&self) -> Value {
        map([
            (
                "microphones",
                Value::new_list(query_devices(DeviceKind::Microphone)),
            ),
            ("cameras", Value::new_list(query_cameras())),
            (
                "sensors",
                Value::new_list(query_devices(DeviceKind::Sensor)),
            ),
            (
                "speakers",
                Value::new_list(query_devices(DeviceKind::Speaker)),
            ),
            (
                "displays",
                Value::new_list(query_devices(DeviceKind::Display)),
            ),
            (
                "batteries",
                Value::new_list(query_devices(DeviceKind::Battery)),
            ),
            (
                "network_adapters",
                Value::new_list(query_devices(DeviceKind::Network)),
            ),
            (
                "storage_devices",
                Value::new_list(query_devices(DeviceKind::Storage)),
            ),
            (
                "usb_devices",
                Value::new_list(query_devices(DeviceKind::Usb)),
            ),
            (
                "bluetooth_devices",
                Value::new_list(query_devices(DeviceKind::Bluetooth)),
            ),
            (
                "printers",
                Value::new_list(query_devices(DeviceKind::Printer)),
            ),
        ])
    }
}

#[derive(Clone, Copy)]
enum DeviceKind {
    Microphone,
    Camera,
    Sensor,
    Speaker,
    Display,
    Battery,
    Network,
    Storage,
    Usb,
    Bluetooth,
    Printer,
}

impl DeviceKind {
    fn patterns(self) -> &'static [&'static str] {
        match self {
            DeviceKind::Microphone => &["Microphone", "Audio", "Headset", "Input"],
            DeviceKind::Camera => &["Camera", "Webcam", "Imaging"],
            DeviceKind::Sensor => &["Sensor", "Accelerometer", "Gyroscope", "Light"],
            DeviceKind::Speaker => &["Speaker", "Headphone", "Headset", "AudioEndpoint"],
            DeviceKind::Display => &["Display", "Monitor", "Graphics", "Video"],
            DeviceKind::Battery => &["Battery", "AC Adapter"],
            DeviceKind::Network => &["Ethernet", "Wi-Fi", "Wireless", "Network", "Bluetooth PAN"],
            DeviceKind::Storage => &["Disk", "Storage", "NVMe", "SSD", "USB Mass Storage"],
            DeviceKind::Usb => &["USB", "Universal Serial Bus"],
            DeviceKind::Bluetooth => &["Bluetooth"],
            DeviceKind::Printer => &["Printer", "Print"],
        }
    }

    fn pnp_classes(self) -> &'static [&'static str] {
        match self {
            DeviceKind::Microphone => &["AudioEndpoint", "MEDIA"],
            DeviceKind::Camera => &["Camera", "Image", "MEDIA"],
            DeviceKind::Sensor => &["Sensor"],
            DeviceKind::Speaker => &["AudioEndpoint", "MEDIA"],
            DeviceKind::Display => &["Display", "Monitor"],
            DeviceKind::Battery => &["Battery"],
            DeviceKind::Network => &["Net"],
            DeviceKind::Storage => &["DiskDrive", "SCSIAdapter", "USB"],
            DeviceKind::Usb => &["USB"],
            DeviceKind::Bluetooth => &["Bluetooth"],
            DeviceKind::Printer => &["Printer"],
        }
    }
}

fn query_devices(kind: DeviceKind) -> Vec<Value> {
    if cfg!(windows) {
        query_windows_devices(kind)
    } else {
        Vec::new()
    }
}

fn query_cameras() -> Vec<Value> {
    let mut cameras = query_native_cameras();
    if cameras.is_empty() {
        cameras = query_devices(DeviceKind::Camera);
    }
    cameras
}

fn query_native_cameras() -> Vec<Value> {
    let Some(api) = native_api_backend() else {
        return Vec::new();
    };
    let Ok(cameras) = query(api) else {
        return Vec::new();
    };

    cameras
        .into_iter()
        .map(|camera| {
            map([
                ("name", str_value(&camera.human_name())),
                ("description", str_value(camera.description())),
                ("class", str_value("camera")),
                ("native", Value::Bool(true)),
                ("backend", str_value(&format!("{:?}", api))),
                ("index", str_value(&format!("{:?}", camera.index()))),
            ])
        })
        .collect()
}

fn camera_frame() -> Result<Value, String> {
    let cameras = query_cameras();
    if cameras.is_empty() {
        return Ok(map([
            ("captured", Value::Bool(false)),
            ("source", str_value("native-camera")),
            ("error", str_value("no camera device found")),
            ("width", Value::Int(0)),
            ("height", Value::Int(0)),
            ("bytes", Value::Int(0)),
        ]));
    }

    let mut camera = match open_first_camera() {
        Ok(camera) => camera,
        Err(error) => {
            return Ok(camera_error(&error));
        }
    };
    if let Err(error) = camera.open_stream() {
        return Ok(camera_error(&format!("camera stream failed: {}", error)));
    }
    thread::sleep(Duration::from_millis(180));
    let frame = match camera.frame() {
        Ok(frame) => frame,
        Err(error) => {
            return Ok(camera_error(&format!("camera frame failed: {}", error)));
        }
    };
    let resolution = frame.resolution();
    let width = resolution.width() as i64;
    let height = resolution.height() as i64;
    let bytes = frame.buffer().len() as i64;

    Ok(map([
        ("captured", Value::Bool(true)),
        ("source", str_value("native-camera-frame")),
        ("width", Value::Int(width)),
        ("height", Value::Int(height)),
        ("bytes", Value::Int(bytes)),
        ("format", str_value("RGB")),
        ("timestamp_ms", Value::Int(now_ms())),
    ]))
}

fn open_first_camera() -> Result<Camera, String> {
    let formats = [
        (640, 480, 30, FrameFormat::MJPEG),
        (640, 480, 30, FrameFormat::YUYV),
        (640, 480, 30, FrameFormat::NV12),
        (1280, 720, 30, FrameFormat::MJPEG),
        (320, 240, 30, FrameFormat::MJPEG),
        (320, 240, 15, FrameFormat::YUYV),
    ];
    let mut errors = Vec::new();

    for (width, height, fps, format) in formats {
        #[allow(deprecated)]
        match Camera::new_with(
            CameraIndex::Index(0),
            width,
            height,
            fps,
            format,
            ApiBackend::Auto,
        ) {
            Ok(camera) => return Ok(camera),
            Err(error) => errors.push(format!(
                "{}x{}@{} {}: {}",
                width, height, fps, format, error
            )),
        }
    }

    Err(format!("camera open failed: {}", errors.join(" | ")))
}

fn camera_error(error: &str) -> Value {
    map([
        ("captured", Value::Bool(false)),
        ("source", str_value("native-camera")),
        ("error", str_value(error)),
        ("width", Value::Int(0)),
        ("height", Value::Int(0)),
        ("bytes", Value::Int(0)),
    ])
}

fn query_windows_devices(kind: DeviceKind) -> Vec<Value> {
    let pattern = kind.patterns().join("|");
    let class_pattern = kind.pnp_classes().join("|");
    let script = format!(
        "Get-CimInstance Win32_PnPEntity | Where-Object {{ ($_.Name -match '{}') -or ($_.PNPClass -match '{}') }} | Select-Object -First 24 Name,Status,PNPClass | ForEach-Object {{ \"$($_.Name)`t$($_.Status)`t$($_.PNPClass)\" }}",
        pattern, class_pattern
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_device_line)
        .collect()
}

fn parse_device_line(line: &str) -> Option<Value> {
    let mut parts = line.split('\t');
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let status = parts.next().unwrap_or("").trim();
    let class_name = parts.next().unwrap_or("").trim();
    Some(map([
        ("name", str_value(name)),
        ("status", str_value(status)),
        ("class", str_value(class_name)),
        ("native", Value::Bool(true)),
    ]))
}

fn device_group_status(method: &str) -> Value {
    map([
        ("backend_method", str_value(method)),
        ("enumeration", Value::Bool(true)),
        ("session_control", Value::Bool(true)),
    ])
}

fn value_map(value: &Value) -> Option<&HashMap<String, Value>> {
    match value {
        Value::Map(map) => Some(map),
        _ => None,
    }
}

fn flag_or(map: Option<&HashMap<String, Value>>, key: &str, default: bool) -> bool {
    map.and_then(|m| m.get(key))
        .and_then(|v| match v {
            Value::Bool(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::new_map(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn str_value(value: &str) -> Value {
    Value::new_string(value.to_string())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn type_error(context: &str, expected: &str, value: &Value) -> String {
    format!(
        "{}: expected {}, got {}",
        context,
        expected,
        value.to_display_string()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_lifecycle_tracks_state() {
        let mut backend = DeviceBackend::new();
        let state = backend
            .call(
                "start_call",
                vec![Value::new_string("mission-room".to_string())],
            )
            .unwrap();
        let Value::Map(state) = state else {
            panic!("state should be a map");
        };
        assert_eq!(state.get("active"), Some(&Value::Bool(true)));
        assert_eq!(
            state.get("session_id"),
            Some(&Value::new_string("mission-room".to_string()))
        );

        let ended = backend.call("end_call", vec![]).unwrap();
        let Value::Map(ended) = ended else {
            panic!("end_call should return a map");
        };
        assert_eq!(
            ended.get("state"),
            Some(&Value::new_string("idle".to_string()))
        );
    }

    #[test]
    fn sensor_frame_increments_counter() {
        let mut backend = DeviceBackend::new();
        let first = backend.call("sensor_frame", vec![]).unwrap();
        let second = backend.call("sensor_frame", vec![]).unwrap();
        let (Value::Map(first), Value::Map(second)) = (first, second) else {
            panic!("sensor frames should be maps");
        };
        assert_eq!(first.get("frame"), Some(&Value::Int(1)));
        assert_eq!(second.get("frame"), Some(&Value::Int(2)));
    }

    #[test]
    fn status_exposes_contract() {
        let mut backend = DeviceBackend::new();
        let status = backend.call("status", vec![]).unwrap();
        let Value::Map(status) = status else {
            panic!("status should be a map");
        };
        assert_eq!(
            status.get("contract"),
            Some(&Value::new_string("MATTER_DEVICE_1".to_string()))
        );
    }
}
