// Copyright (C) 2026 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    any::Any,
    sync::mpsc::{Receiver, Sender, channel},
    thread::{self, JoinHandle},
};

pub struct Processor<J, R> {
    rx: Receiver<R>,
    tx: Option<Sender<J>>,
    thread: Option<JoinHandle<anyhow::Result<()>>>,
}

impl<J, R> Processor<J, R>
where
    J: Send + 'static,
    R: Send + 'static,
{
    pub fn new<F>(processor: F) -> anyhow::Result<Self>
    where
        F: Send + 'static + FnOnce(Receiver<J>, Sender<R>) -> anyhow::Result<()>,
    {
        let (txj, rxj) = channel();
        let (txr, rxr) = channel();
        Ok(Self {
            rx: rxr,
            tx: Some(txj),
            thread: Some(
                thread::Builder::new()
                    .name("L0ttie Renderer".into())
                    .spawn(move || processor(rxj, txr))?,
            ),
        })
    }

    pub fn process(&mut self, job: J) -> anyhow::Result<R> {
        if let Some(ref tx) = self.tx {
            tx.send(job).map_err(|_| self.join_error())?;
            self.rx.recv().map_err(|_| self.join_error())
        } else {
            Err(anyhow::anyhow!("Unable to send to processor"))
        }
    }

    fn join_error(&mut self) -> anyhow::Error {
        match self.thread.take() {
            Some(thread) => match thread.join() {
                Ok(Err(err)) => err,
                Ok(Ok(())) => anyhow::anyhow!("Failed to process render"),
                Err(payload) => panic_error(payload),
            },
            None => anyhow::anyhow!("Failed to process render"),
        }
    }
}

impl<J, R> Drop for Processor<J, R> {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take()
            && let Some(thread) = self.thread.take()
        {
            drop(tx);
            let _ = thread.join();
        }
    }
}

fn panic_error(payload: Box<dyn Any + Send>) -> anyhow::Error {
    match payload.downcast::<String>() {
        Ok(message) => anyhow::anyhow!("Worker thread panicked: {message}"),
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => anyhow::anyhow!("Worker thread panicked: {message}"),
            Err(_) => anyhow::anyhow!("Worker thread panicked"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Job(i32);
    #[derive(Debug, PartialEq)]
    struct Response(f32);

    #[test]
    fn test_process() {
        let mut processor = Processor::new(|rx: Receiver<Job>, tx: Sender<Response>| {
            for job in rx {
                tx.send(Response(job.0 as f32))?;
            }
            Ok(())
        })
        .unwrap();

        assert_eq!(processor.process(Job(7)).unwrap(), Response(7.0));
    }
}
