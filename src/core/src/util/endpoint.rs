use std::sync::mpsc::{channel, Receiver, RecvError, SendError, Sender};

pub struct Endpoint<S, R> {
    sender: Sender<S>,
    receiver: Receiver<R>,
}

impl<S, R> Endpoint<S, R>
where
    S: Send,
    R: Send,
{
    pub fn pair() -> (Endpoint<S, R>, Endpoint<R, S>) {
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let e1 = Endpoint {
            sender: tx1,
            receiver: rx2,
        };
        let e2 = Endpoint {
            sender: tx2,
            receiver: rx1,
        };

        (e1, e2)
    }

    pub fn send(&self, s: S) -> Result<(), SendError<S>> {
        self.sender.send(s)
    }

    pub fn recv(&self) -> Result<R, RecvError> {
        self.receiver.recv()
    }
}
