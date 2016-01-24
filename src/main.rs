
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use std::io::{Read, Write};

// How often to poke at the client connection to make sure it's alive.
//
// We use a pretty conservative value here since our main client is a mobile phone app; no reason
// to kill its battery life just because we're insecure...
const KEEPALIVE_SECONDS: u64 = 4 * 60;

// The file to create/remove to stay awake.
//
// This file doesn't actually do anything itself, it's just checked by other processes elsewhere in
// our system.
const WAKELOCK_FILE: &'static str = "/tmp/wakelock";

// Keeps count of the number of active clients (by receiving +1 / -1 messages) and creates/removes
// a magic wakelock file based on if there's at least one client active.
fn manage_wakelock(rx: Receiver<i64>) {
    let mut num_clients = 0i64;

    // Get the initial state set up properly.
    let _ = std::fs::remove_file(WAKELOCK_FILE);

    // Listen to client add/drops and update state as appropriate.
    loop {
        num_clients += rx.recv().unwrap();
        match num_clients {
            0 => {
                println!("Bedtime!");
                std::fs::remove_file(WAKELOCK_FILE).unwrap()
            },
            n if n > 0 => {
                println!("{} clients connected; staying up...", n);
                std::fs::OpenOptions::new().create(true).open(WAKELOCK_FILE).unwrap();
            },
            _ => panic!("Flaws in space-time detected--negative clients!")
        }
    }
}

// Manages the state machine for each connected client.
//
// A client is expected to keep up a continuous (but very low bandwidth) stream of network traffic
// in order to keep the machine awake.  In case we're not hearing anything from the client, we
// occasionally poke the connection with a byte to ensure that it's still alive; this can
// accelerate TCP disconnect detection.
//
// The simplest possible client is actually just an echo client--simply responding to the poke byte
// is a completely valid way to stay connected.
fn handle_client(mut stream: TcpStream, tx: Sender<i64>) {
    let peer = stream.peer_addr().unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(KEEPALIVE_SECONDS))).unwrap();
    stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write("hi".as_bytes()).unwrap();

    println!("New client {}!", peer);
    tx.send(1).unwrap();

    let mut staleness = 0;
    let mut buffer = [0; 2048];

    while staleness <= 5 {
        match stream.read(&mut buffer[..]) {
            // If the peer disconnected, we'll get 0 bytes.
            Ok(0) => break,

            // If the peer sent something, they're good to go.
            Ok(_) => {
                println!("{} is alive.", peer);
                staleness = 0;
            },

            // If we haven't heard anything lately, send a keep-alive poke.
            Err(_) => {
                println!("Poking {}...", peer);
                match stream.write(".".as_bytes()) {
                    Ok(_) => staleness += 1,

                    // Socket error; kill off this client.
                    _ => break,
                }
            },
        }
    }

    tx.send(-1).unwrap();
    println!("{} is dead, long live {}!", peer, peer);
}

fn listen_for_clients(tx: Sender<i64>) {
    let listener = TcpListener::bind("0.0.0.0:5005").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx = tx.clone();
                thread::spawn(move|| {
                    handle_client(stream, tx)
                });
            }
            _ => break
        }
    }
}

fn main() {
    let (tx, rx) = channel();

    thread::spawn(move|| {
        listen_for_clients(tx);
    });

    manage_wakelock(rx);
}
