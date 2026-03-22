use audio_relay::{
    AudioLayout, AudioReceiver, AudioSender, CaptureConfig, CodecConfig, DEFAULT_INITIAL_DROP_MS,
    DEFAULT_PACKET_DURATION_MS, Error, PlaybackConfig, ReceiverConfig, Result, SenderConfig,
};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return Err(Error::InvalidArgument("missing subcommand".into()));
    }

    let command = args.remove(0);
    match command.as_str() {
        "send" => run_send(args),
        "recv" | "receive" => run_receive(args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(Error::InvalidArgument(format!(
            "unknown subcommand: {other}"
        ))),
    }
}

fn run_send(args: Vec<String>) -> Result<()> {
    let mut bind_addr = SocketAddr::from_str("0.0.0.0:0").unwrap();
    let mut destination = None;
    let mut device_name = None;
    let mut packet_duration_ms = DEFAULT_PACKET_DURATION_MS;
    let mut high_quality = false;
    let mut no_fec = false;
    let mut layout = AudioLayout::Stereo;
    let mut timeout_ms = 5_000u64;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--to" => destination = Some(parse_socket_addr(expect_value(&mut iter, "--to")?)?),
            "--bind" => bind_addr = parse_socket_addr(expect_value(&mut iter, "--bind")?)?,
            "--device" => device_name = Some(expect_value(&mut iter, "--device")?),
            "--packet-ms" => {
                packet_duration_ms = expect_value(&mut iter, "--packet-ms")?
                    .parse()
                    .map_err(|_| Error::InvalidArgument("invalid --packet-ms value".into()))?
            }
            "--timeout-ms" => {
                timeout_ms = expect_value(&mut iter, "--timeout-ms")?
                    .parse()
                    .map_err(|_| Error::InvalidArgument("invalid --timeout-ms value".into()))?
            }
            "--high-quality" => high_quality = true,
            "--no-fec" => no_fec = true,
            "--layout" => layout = parse_layout(&expect_value(&mut iter, "--layout")?)?,
            flag => {
                return Err(Error::InvalidArgument(format!(
                    "unknown flag for send: {flag}"
                )));
            }
        }
    }

    let destination = destination
        .ok_or_else(|| Error::InvalidArgument("missing required --to host:port".into()))?;
    let config = SenderConfig {
        bind_addr,
        destination,
        codec: CodecConfig {
            layout,
            packet_duration_ms,
            high_quality,
        },
        capture: CaptureConfig { device_name },
        read_timeout: Duration::from_millis(timeout_ms),
        enable_fec: !no_fec,
        ssrc: 0,
    };

    let mut sender = AudioSender::bind(config)?;
    sender.run()
}

fn run_receive(args: Vec<String>) -> Result<()> {
    let mut bind_addr = SocketAddr::from_str("0.0.0.0:48000").unwrap();
    let mut packet_duration_ms = DEFAULT_PACKET_DURATION_MS;
    let mut high_quality = false;
    let mut layout = AudioLayout::Stereo;
    let mut timeout_ms = 5_000u64;
    let mut initial_drop_ms = DEFAULT_INITIAL_DROP_MS;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--listen" => bind_addr = parse_socket_addr(expect_value(&mut iter, "--listen")?)?,
            "--packet-ms" => {
                packet_duration_ms = expect_value(&mut iter, "--packet-ms")?
                    .parse()
                    .map_err(|_| Error::InvalidArgument("invalid --packet-ms value".into()))?
            }
            "--timeout-ms" => {
                timeout_ms = expect_value(&mut iter, "--timeout-ms")?
                    .parse()
                    .map_err(|_| Error::InvalidArgument("invalid --timeout-ms value".into()))?
            }
            "--initial-drop-ms" => {
                initial_drop_ms = expect_value(&mut iter, "--initial-drop-ms")?
                    .parse()
                    .map_err(|_| Error::InvalidArgument("invalid --initial-drop-ms value".into()))?
            }
            "--high-quality" => high_quality = true,
            "--layout" => layout = parse_layout(&expect_value(&mut iter, "--layout")?)?,
            flag => {
                return Err(Error::InvalidArgument(format!(
                    "unknown flag for recv: {flag}"
                )));
            }
        }
    }

    let config = ReceiverConfig {
        bind_addr,
        codec: CodecConfig {
            layout,
            packet_duration_ms,
            high_quality,
        },
        playback: PlaybackConfig { device_name: None },
        read_timeout: Duration::from_millis(timeout_ms),
        initial_drop_ms,
    };

    let mut receiver = AudioReceiver::bind(config)?;
    receiver.run()
}

fn expect_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    iter.next()
        .ok_or_else(|| Error::InvalidArgument(format!("missing value for {flag}")))
}

fn parse_socket_addr(value: String) -> Result<SocketAddr> {
    value
        .parse()
        .map_err(|_| Error::InvalidArgument(format!("invalid socket address: {value}")))
}

fn parse_layout(value: &str) -> Result<AudioLayout> {
    match value {
        "stereo" | "2" | "2.0" => Ok(AudioLayout::Stereo),
        "5.1" | "51" | "surround51" => Ok(AudioLayout::Surround51),
        "7.1" | "71" | "surround71" => Ok(AudioLayout::Surround71),
        _ => Err(Error::InvalidArgument(format!(
            "unsupported audio layout: {value}"
        ))),
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  audio-relay send --to HOST:PORT [--bind HOST:PORT] [--layout stereo|5.1|7.1] [--packet-ms 5] [--high-quality] [--no-fec]
  audio-relay recv [--listen HOST:PORT] [--layout stereo|5.1|7.1] [--packet-ms 5] [--high-quality] [--initial-drop-ms 500]"
    );
}
