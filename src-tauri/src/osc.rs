use rosc::{OscMessage, OscPacket, OscType};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum HudEvent {
    Single {
        message: String,
        color: Option<String>,
    },
    Lines {
        messages: Vec<String>,
        color: Option<String>,
    },
    Flash {
        message: String,
        duration_s: f32,
        color: Option<String>,
    },
    Clear,
    SetColor {
        color: String,
    },
    SetBackground {
        color: String,
    },
    SetFontSize {
        size: u32,
    },
}

const NAMED_COLORS: &[&str] = &[
    "white", "red", "green", "blue", "yellow", "teal", "orange", "purple", "pink",
];

pub(crate) fn is_color(s: &str) -> bool {
    if s.starts_with('#') && (s.len() == 4 || s.len() == 7) {
        return s[1..].chars().all(|c| c.is_ascii_hexdigit());
    }
    NAMED_COLORS.contains(&s.to_lowercase().as_str())
}

pub(crate) fn extract_strings(args: &[OscType]) -> Vec<String> {
    args.iter()
        .filter_map(|a| match a {
            OscType::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

pub(crate) fn extract_float(arg: &OscType) -> Option<f32> {
    match arg {
        OscType::Float(f) => Some(*f),
        OscType::Double(d) => Some(*d as f32),
        OscType::Int(i) => Some(*i as f32),
        _ => None,
    }
}

pub(crate) fn parse_osc_message(msg: &OscMessage) -> Option<HudEvent> {
    match msg.addr.as_str() {
        "/sndwrks/hud/message/single" => {
            let strings = extract_strings(&msg.args);
            if strings.is_empty() {
                return None;
            }
            let (message, color) = if strings.len() >= 2 && is_color(&strings[strings.len() - 1]) {
                (strings[0].clone(), Some(strings[strings.len() - 1].clone()))
            } else {
                (strings[0].clone(), None)
            };
            Some(HudEvent::Single { message, color })
        }
        "/sndwrks/hud/message/lines" => {
            let strings = extract_strings(&msg.args);
            if strings.is_empty() {
                return None;
            }
            let (messages, color) = if strings.len() >= 2 && is_color(&strings[strings.len() - 1])
            {
                (
                    strings[..strings.len() - 1].to_vec(),
                    Some(strings[strings.len() - 1].clone()),
                )
            } else {
                (strings, None)
            };
            Some(HudEvent::Lines { messages, color })
        }
        "/sndwrks/hud/message/flash" => {
            if msg.args.len() < 2 {
                return None;
            }
            let message = match &msg.args[0] {
                OscType::String(s) => s.clone(),
                _ => return None,
            };
            let duration_s = match extract_float(&msg.args[1]) {
                Some(f) => f,
                None => return None,
            };
            let color = msg
                .args
                .get(2)
                .and_then(|a| match a {
                    OscType::String(s) => Some(s.clone()),
                    _ => None,
                })
                .filter(|s| is_color(s));
            Some(HudEvent::Flash {
                message,
                duration_s,
                color,
            })
        }
        "/sndwrks/hud/clear" => Some(HudEvent::Clear),
        "/sndwrks/hud/color" => {
            let strings = extract_strings(&msg.args);
            if strings.is_empty() {
                return None;
            }
            Some(HudEvent::SetColor {
                color: strings[0].clone(),
            })
        }
        "/sndwrks/hud/background" => {
            let strings = extract_strings(&msg.args);
            if strings.is_empty() {
                return None;
            }
            Some(HudEvent::SetBackground {
                color: strings[0].clone(),
            })
        }
        "/sndwrks/hud/fontsize" => {
            let size = msg.args.first().and_then(|a| match a {
                OscType::Int(i) => Some(*i as u32),
                OscType::Float(f) => Some(*f as u32),
                _ => None,
            });
            size.map(|s| HudEvent::SetFontSize { size: s })
        }
        _ => None,
    }
}

fn handle_message(msg: OscMessage, app: &AppHandle) {
    if let Some(event) = parse_osc_message(&msg) {
        if let Err(e) = app.emit("hud-update", &event) {
            eprintln!("Failed to emit hud-update event: {}", e);
        }
    }
}

fn handle_packet(packet: OscPacket, app: &AppHandle) {
    match packet {
        OscPacket::Message(msg) => handle_message(msg, app),
        OscPacket::Bundle(bundle) => {
            for p in bundle.content {
                handle_packet(p, app);
            }
        }
    }
}

pub async fn start_udp_listener(port: u16, app: AppHandle) {
    let addr = format!("0.0.0.0:{}", port);
    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => {
            println!("OSC UDP listener started on port {}", port);
            s
        }
        Err(e) => {
            eprintln!("Failed to bind UDP socket on {}: {}", addr, e);
            return;
        }
    };

    let mut buf = [0u8; 4096];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, _addr)) => {
                if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..len]) {
                    handle_packet(packet, &app);
                }
            }
            Err(e) => {
                eprintln!("UDP recv error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::OscType;

    fn msg(addr: &str, args: Vec<OscType>) -> OscMessage {
        OscMessage {
            addr: addr.to_string(),
            args,
        }
    }

    // ── is_color ──

    #[test]
    fn is_color_valid_hex_short() {
        assert!(is_color("#FFF"));
        assert!(is_color("#abc"));
    }

    #[test]
    fn is_color_valid_hex_long() {
        assert!(is_color("#ffffff"));
        assert!(is_color("#aabbcc"));
    }

    #[test]
    fn is_color_invalid_hex() {
        assert!(!is_color("#GGG"));
        assert!(!is_color("#12"));
        assert!(!is_color("#1234567"));
        assert!(!is_color("#"));
        assert!(!is_color(""));
    }

    #[test]
    fn is_color_named_colors() {
        for name in NAMED_COLORS {
            assert!(is_color(name), "{} should be a valid color", name);
        }
    }

    #[test]
    fn is_color_case_insensitive() {
        assert!(is_color("Red"));
        assert!(is_color("RED"));
        assert!(is_color("White"));
    }

    #[test]
    fn is_color_non_colors() {
        assert!(!is_color("hello"));
        assert!(!is_color("reddish"));
    }

    // ── extract_strings ──

    #[test]
    fn extract_strings_mixed_types() {
        let args = vec![
            OscType::String("hello".into()),
            OscType::Int(42),
            OscType::String("world".into()),
            OscType::Float(1.0),
        ];
        assert_eq!(extract_strings(&args), vec!["hello", "world"]);
    }

    #[test]
    fn extract_strings_empty() {
        assert!(extract_strings(&[]).is_empty());
    }

    // ── extract_float ──

    #[test]
    fn extract_float_from_float() {
        assert_eq!(extract_float(&OscType::Float(1.5)), Some(1.5));
    }

    #[test]
    fn extract_float_from_double() {
        assert_eq!(extract_float(&OscType::Double(2.5)), Some(2.5));
    }

    #[test]
    fn extract_float_from_int() {
        assert_eq!(extract_float(&OscType::Int(3)), Some(3.0));
    }

    #[test]
    fn extract_float_from_string() {
        assert_eq!(extract_float(&OscType::String("x".into())), None);
    }

    // ── parse_osc_message: /sndwrks/hud/message/single ──

    #[test]
    fn single_one_string() {
        let m = msg("/sndwrks/hud/message/single", vec![OscType::String("Hello".into())]);
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Single { message: "Hello".into(), color: None })
        );
    }

    #[test]
    fn single_with_color() {
        let m = msg(
            "/sndwrks/hud/message/single",
            vec![OscType::String("STANDBY".into()), OscType::String("red".into())],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Single { message: "STANDBY".into(), color: Some("red".into()) })
        );
    }

    #[test]
    fn single_second_arg_not_color() {
        let m = msg(
            "/sndwrks/hud/message/single",
            vec![OscType::String("Hello".into()), OscType::String("notacolor".into())],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Single { message: "Hello".into(), color: None })
        );
    }

    #[test]
    fn single_no_strings() {
        let m = msg("/sndwrks/hud/message/single", vec![OscType::Int(42)]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── parse_osc_message: /sndwrks/hud/message/lines ──

    #[test]
    fn lines_multiple_strings() {
        let m = msg(
            "/sndwrks/hud/message/lines",
            vec![
                OscType::String("Line 1".into()),
                OscType::String("Line 2".into()),
                OscType::String("Line 3".into()),
            ],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Lines {
                messages: vec!["Line 1".into(), "Line 2".into(), "Line 3".into()],
                color: None,
            })
        );
    }

    #[test]
    fn lines_with_color() {
        let m = msg(
            "/sndwrks/hud/message/lines",
            vec![
                OscType::String("ACT II".into()),
                OscType::String("Scene 3".into()),
                OscType::String("yellow".into()),
            ],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Lines {
                messages: vec!["ACT II".into(), "Scene 3".into()],
                color: Some("yellow".into()),
            })
        );
    }

    #[test]
    fn lines_single_string() {
        let m = msg("/sndwrks/hud/message/lines", vec![OscType::String("Solo".into())]);
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Lines { messages: vec!["Solo".into()], color: None })
        );
    }

    #[test]
    fn lines_empty() {
        let m = msg("/sndwrks/hud/message/lines", vec![]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── parse_osc_message: /sndwrks/hud/message/flash ──

    #[test]
    fn flash_string_and_float() {
        let m = msg(
            "/sndwrks/hud/message/flash",
            vec![OscType::String("GO".into()), OscType::Float(2.0)],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Flash { message: "GO".into(), duration_s: 2.0, color: None })
        );
    }

    #[test]
    fn flash_with_color() {
        let m = msg(
            "/sndwrks/hud/message/flash",
            vec![
                OscType::String("GO".into()),
                OscType::Float(1.5),
                OscType::String("red".into()),
            ],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Flash {
                message: "GO".into(),
                duration_s: 1.5,
                color: Some("red".into()),
            })
        );
    }

    #[test]
    fn flash_with_non_color_third_arg() {
        let m = msg(
            "/sndwrks/hud/message/flash",
            vec![
                OscType::String("GO".into()),
                OscType::Float(1.0),
                OscType::String("notacolor".into()),
            ],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Flash { message: "GO".into(), duration_s: 1.0, color: None })
        );
    }

    #[test]
    fn flash_int_duration() {
        let m = msg(
            "/sndwrks/hud/message/flash",
            vec![OscType::String("GO".into()), OscType::Int(3)],
        );
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::Flash { message: "GO".into(), duration_s: 3.0, color: None })
        );
    }

    #[test]
    fn flash_too_few_args() {
        let m = msg("/sndwrks/hud/message/flash", vec![OscType::String("GO".into())]);
        assert_eq!(parse_osc_message(&m), None);
    }

    #[test]
    fn flash_first_arg_not_string() {
        let m = msg("/sndwrks/hud/message/flash", vec![OscType::Int(1), OscType::Float(1.0)]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── parse_osc_message: /sndwrks/hud/clear ──

    #[test]
    fn clear() {
        let m = msg("/sndwrks/hud/clear", vec![]);
        assert_eq!(parse_osc_message(&m), Some(HudEvent::Clear));
    }

    #[test]
    fn clear_with_args() {
        let m = msg("/sndwrks/hud/clear", vec![OscType::Int(1)]);
        assert_eq!(parse_osc_message(&m), Some(HudEvent::Clear));
    }

    // ── parse_osc_message: /sndwrks/hud/color ──

    #[test]
    fn set_color() {
        let m = msg("/sndwrks/hud/color", vec![OscType::String("#ff0000".into())]);
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::SetColor { color: "#ff0000".into() })
        );
    }

    #[test]
    fn set_color_no_args() {
        let m = msg("/sndwrks/hud/color", vec![]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── parse_osc_message: /sndwrks/hud/background ──

    #[test]
    fn set_background() {
        let m = msg("/sndwrks/hud/background", vec![OscType::String("blue".into())]);
        assert_eq!(
            parse_osc_message(&m),
            Some(HudEvent::SetBackground { color: "blue".into() })
        );
    }

    #[test]
    fn set_background_no_args() {
        let m = msg("/sndwrks/hud/background", vec![]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── parse_osc_message: /sndwrks/hud/fontsize ──

    #[test]
    fn set_fontsize_int() {
        let m = msg("/sndwrks/hud/fontsize", vec![OscType::Int(48)]);
        assert_eq!(parse_osc_message(&m), Some(HudEvent::SetFontSize { size: 48 }));
    }

    #[test]
    fn set_fontsize_float() {
        let m = msg("/sndwrks/hud/fontsize", vec![OscType::Float(72.0)]);
        assert_eq!(parse_osc_message(&m), Some(HudEvent::SetFontSize { size: 72 }));
    }

    #[test]
    fn set_fontsize_no_args() {
        let m = msg("/sndwrks/hud/fontsize", vec![]);
        assert_eq!(parse_osc_message(&m), None);
    }

    #[test]
    fn set_fontsize_string_arg() {
        let m = msg("/sndwrks/hud/fontsize", vec![OscType::String("big".into())]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── unknown address ──

    #[test]
    fn unknown_address() {
        let m = msg("/unknown/address", vec![OscType::String("hi".into())]);
        assert_eq!(parse_osc_message(&m), None);
    }

    // ── HudEvent serialization ──

    #[test]
    fn serialize_single() {
        let event = HudEvent::Single { message: "test".into(), color: Some("red".into()) };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "Single");
        assert_eq!(json["message"], "test");
        assert_eq!(json["color"], "red");
    }

    #[test]
    fn serialize_clear() {
        let json = serde_json::to_value(&HudEvent::Clear).unwrap();
        assert_eq!(json["type"], "Clear");
    }

    #[test]
    fn serialize_lines() {
        let event = HudEvent::Lines {
            messages: vec!["a".into(), "b".into()],
            color: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "Lines");
        assert_eq!(json["messages"], serde_json::json!(["a", "b"]));
        assert_eq!(json["color"], serde_json::Value::Null);
    }

    #[test]
    fn serialize_flash() {
        let event = HudEvent::Flash {
            message: "GO".into(),
            duration_s: 1.5,
            color: Some("#ff0000".into()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "Flash");
        assert_eq!(json["message"], "GO");
        assert_eq!(json["duration_s"], 1.5);
        assert_eq!(json["color"], "#ff0000");
    }

    #[test]
    fn serialize_set_fontsize() {
        let json = serde_json::to_value(&HudEvent::SetFontSize { size: 48 }).unwrap();
        assert_eq!(json["type"], "SetFontSize");
        assert_eq!(json["size"], 48);
    }
}
