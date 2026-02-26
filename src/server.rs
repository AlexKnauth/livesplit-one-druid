use std::net::TcpListener;
use std::thread::spawn;
// use futures_util::FutureExt;
use tungstenite::accept;

use livesplit_core::{event, networking::server_protocol};

pub fn server_start<S: event::CommandSink + event::TimerQuery + Clone + Send + 'static>(
    port: i64,
    command_sink: S,
) {
    spawn(move || server_main(port, command_sink));
}

/// A WebSocket echo server
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
            let Ok(mut websocket) = accept(stream) else {
                return;
            };
            loop {
                let Ok(msg) = websocket.read() else {
                    websocket.close(None).ok();
                    return;
                };

                // We do not want to send back ping/pong messages.
                if msg.is_binary() || msg.is_text() {
                    let r = futures::executor::block_on(server_protocol::handle_command(
                        msg.to_text().unwrap(),
                        &command_sink,
                    ));
                    websocket.send(r.into()).unwrap();
                }
            }
        });
    }
}
