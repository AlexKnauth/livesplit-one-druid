use std::io::{BufRead, BufWriter, Write};
use std::thread::spawn;
use std::{io::BufReader, net::TcpListener};
// use futures_util::FutureExt;

use livesplit_core::{event, networking::server_protocol};

pub fn server_start<S: event::CommandSink + event::TimerQuery + Clone + Send + 'static>(
    port: i64,
    command_sink: S,
) {
    spawn(move || server_main(port, command_sink));
}

fn server_main<S: event::CommandSink + event::TimerQuery + Clone + Send + 'static>(
    port: i64,
    command_sink: S,
) {
    let server = TcpListener::bind(&format!("127.0.0.1:{port}")).unwrap();
    for stream in server.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let command_sink = command_sink.clone();
        spawn(move || {
            let mut writer = BufWriter::new(&stream);
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            loop {
                let Ok(msg) = reader.read_line(&mut line).map(|_| line.trim_end_matches(['\n', '\r'])) else {
                    stream.shutdown(std::net::Shutdown::Both).ok();
                    return;
                };

                {
                    let r = futures::executor::block_on(server_protocol::handle_command(
                        msg,
                        &command_sink,
                    ));
                    if !r.is_empty() {
                        writer.write_fmt(format_args!("{}\n", r)).unwrap();
                        writer.flush().unwrap();
                    }
                }
            }
        });
    }
}
