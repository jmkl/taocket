use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::{Bytes, Utf8Bytes};
use tokio_tungstenite::{accept_async, tungstenite};

#[derive(Debug)]
pub enum Error {
    FailedToStart,
}

#[derive(Debug, Clone)]
pub enum Message {
    Text(String),
}

impl Message {
    fn into_tungstenite(self) -> tungstenite::Message {
        match self {
            Self::Text(text) => tungstenite::Message::Text(Utf8Bytes::from(text)),
        }
    }

    fn from_tungstenite(message: tungstenite::Message) -> Option<Self> {
        match message {
            tungstenite::Message::Text(s) => Some(Message::Text(s.to_string())),
            _ => None,
        }
    }
}

enum ResponderCommand {
    Message(Message),
    CloseConnection,
}

///

///

#[derive(Debug, Clone)]
pub struct Responder {
    tx: flume::Sender<ResponderCommand>,
    client_id: u64,
}

impl Responder {
    fn new(tx: flume::Sender<ResponderCommand>, client_id: u64) -> Self {
        Self { tx, client_id }
    }

    ///

    ///

    pub fn send(&self, message: Message) -> bool {
        self.tx.send(ResponderCommand::Message(message)).is_ok()
    }

    ///

    pub fn close(&self) {
        let _ = self.tx.send(ResponderCommand::CloseConnection);
    }

    pub fn client_id(&self) -> u64 {
        self.client_id
    }
}

#[derive(Debug)]
pub enum Event {
    Connect(u64, Responder),

    Disconnect(u64),

    Message(u64, Message),
}

///

#[derive(Debug)]
pub struct EventHub {
    rx: flume::Receiver<Event>,
}

impl EventHub {
    fn new(rx: flume::Receiver<Event>) -> Self {
        Self { rx }
    }

    pub fn drain(&self) -> Vec<Event> {
        if self.rx.is_disconnected() && self.rx.is_empty() {
            panic!(
                "EventHub channel disconnected. Panicking because Websocket listener thread was killed."
            );
        }

        self.rx.drain().collect()
    }

    pub fn next_event(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    pub fn poll_event(&self) -> Event {
        self.rx.recv().unwrap()
    }

    pub async fn poll_async(&self) -> Event {
        self.rx.recv_async().await.expect("Parent thread is dead")
    }

    pub fn is_empty(&self) -> bool {
        self.rx.is_empty()
    }
}

pub fn launch(port: u16) -> Result<EventHub, Error> {
    let address = format!("0.0.0.0:{}", port);
    let listener = std::net::TcpListener::bind(&address).map_err(|_| Error::FailedToStart)?;
    return launch_from_listener(listener);
}

///

///

pub fn launch_from_listener(listener: std::net::TcpListener) -> Result<EventHub, Error> {
    let (tx, rx) = flume::unbounded();
    std::thread::Builder::new()
        .name("Websocket listener".to_string())
        .spawn(move || {
            start_runtime(tx, listener).unwrap();
        })
        .map_err(|_| Error::FailedToStart)?;

    Ok(EventHub::new(rx))
}

fn start_runtime(
    event_tx: flume::Sender<Event>,
    listener: std::net::TcpListener,
) -> Result<(), Error> {
    listener
        .set_nonblocking(true)
        .map_err(|_| Error::FailedToStart)?;
    Runtime::new()
        .map_err(|_| Error::FailedToStart)?
        .block_on(async {
            let tokio_listener = TcpListener::from_std(listener).unwrap();
            let mut current_id: u64 = 0;
            loop {
                match tokio_listener.accept().await {
                    Ok((stream, _)) => {
                        tokio::spawn(handle_connection(stream, event_tx.clone(), current_id));
                        current_id = current_id.wrapping_add(1);
                    }
                    _ => {}
                }
            }
        })
}

async fn handle_connection(stream: TcpStream, event_tx: flume::Sender<Event>, id: u64) {
    let ws_stream = match accept_async(stream).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let (mut outgoing, mut incoming) = ws_stream.split();

    // channel for the `Responder` to send things to this websocket
    let (resp_tx, resp_rx) = flume::unbounded();

    event_tx
        .send(Event::Connect(id, Responder::new(resp_tx, id)))
        .expect("Parent thread is dead");

    // future that waits for commands from the `Responder`
    let responder_events = async move {
        while let Ok(event) = resp_rx.recv_async().await {
            match event {
                ResponderCommand::Message(message) => {
                    if let Err(_) = outgoing.send(message.into_tungstenite()).await {
                        let _ = outgoing.close().await;
                        return Ok(());
                    }
                }
                ResponderCommand::CloseConnection => {
                    let _ = outgoing.close().await;
                    return Ok(());
                }
            }
        }

        // Disconnect if the `Responder` was dropped without explicitly disconnecting
        let _ = outgoing.close().await;

        // this future always returns Ok, so that it wont stop the try_join
        Result::<(), ()>::Ok(())
    };

    let event_tx2 = event_tx.clone();
    //future that forwards messages received from the websocket to the event channel
    let events = async move {
        while let Some(message) = incoming.next().await {
            if let Ok(tungstenite_msg) = message {
                if let Some(msg) = Message::from_tungstenite(tungstenite_msg) {
                    event_tx2
                        .send(Event::Message(id, msg))
                        .expect("Parent thread is dead");
                }
            }
        }

        // stop the try_join once the websocket is closed and all pending incoming
        // messages have been sent to the event channel.
        // stopping the try_join causes responder_events to be closed too so that the
        // `Receiver` cant send any more messages.
        Result::<(), ()>::Err(())
    };

    // use try_join so that when `events` returns Err (the websocket closes), responder_events will be stopped too
    let _ = futures_util::try_join!(responder_events, events);

    event_tx
        .send(Event::Disconnect(id))
        .expect("Parent thread is dead");
}
