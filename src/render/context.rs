use super::*;

use std::{
    any::Any,
    io,
    ops::{Deref, DerefMut},
    pin::Pin,
};
use tokio::sync::{mpsc, oneshot};

/// Defines visual stacking order for nested child widgets relative to the parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubWidgetPosition {
    /// Render subview ON TOP OF the current one (the parent is shifted down).
    Above,
    /// Render subview FROM the BOTTOM of the current one (the parent remains fixed at the top).
    Below,
    /// Temporarily ERASE the current widget and render the sub-widget in its place (modal mode).
    Replace,
}

/// Control messages sent from `Context` to manage renderer lifecycle and execution flow.
pub enum Command {
    Invalidate,
    Freeze,
    Resume,
    PushSubWidget {
        runner: Box<
            dyn for<'a> FnOnce(
                    &'a mut io::BufWriter<io::Stdout>,
                    usize,
                ) -> Pin<
                    Box<
                        dyn std::future::Future<
                                Output = io::Result<(Box<dyn Any + Send>, Vec<String>)>,
                            > + Send
                            + 'a,
                    >,
                > + Send,
        >,
        position: SubWidgetPosition,
        keep_rendered: bool,
        reply: oneshot::Sender<Box<dyn Any + Send>>,
    },
    Finish,
}

pub struct Context<T: Clone + Send + Sync + 'static, E: Send + 'static> {
    pub state: atoman::StateGuard<T>,
    pub tx: mpsc::UnboundedSender<Command>,
    pub event_tx: mpsc::UnboundedSender<E>,
}

impl<T: Clone + Send + Sync + 'static, E: Send + 'static> Context<T, E> {
    pub fn sync(&mut self) {
        self.state.sync();
    }

    pub fn sync_n(&mut self, n: usize) {
        self.state.sync_n(n);
    }

    pub fn notify(&mut self) {
        self.state.sync();
        self.send_command(Command::Invalidate);
    }

    pub fn send_event(&self, event: E) {
        let _ = self.event_tx.send(event);
    }

    pub fn send_command(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    pub fn freeze(&self) {
        self.send_command(Command::Freeze);
    }

    pub fn resume(&self) {
        self.send_command(Command::Resume);
    }

    pub fn finish(&mut self) {
        self.state.sync();
        self.send_command(Command::Finish);
    }

    /// Runs nested `Block<SubW>` and returns its result.
    ///
    /// # Параметры
    /// - `sub_block`: Configured instance `Block<SubW>`.
    /// - `position`: Rendering rule (`Above`, `Below`, `Replace`).
    /// - `keep_rendered`: Leave frame of the subwidget on the screen after completion.
    pub async fn run_sub_widget<SubW>(
        &self,
        sub_block: Block<SubW>,
        position: SubWidgetPosition,
        keep_rendered: bool,
    ) -> io::Result<SubW::Output>
    where
        SubW: Widget + Send + 'static,
        SubW::Output: Send + 'static,
        SubW::State: Send + Sync + 'static,
        SubW::Event: Send + 'static,
    {
        self.push_sub_widget(position, keep_rendered, move |writer, _prev_h| {
            Box::pin(async move {
                let (output, lines) = sub_block.render_with_writer(writer).await?;
                let boxed_output: Box<dyn Any + Send> = Box::new(output);
                Ok((boxed_output, lines))
            })
        })
        .await
    }

    /// Simplified method for displaying a sub-widget (by default, it blurs its frame after exiting).
    pub async fn show_sub_widget<SubW>(
        &self,
        sub_block: Block<SubW>,
        position: SubWidgetPosition,
    ) -> io::Result<SubW::Output>
    where
        SubW: Widget + Send + 'static,
        SubW::Output: Send + 'static,
        SubW::State: Send + Sync + 'static,
        SubW::Event: Send + 'static,
    {
        self.run_sub_widget(sub_block, position, false).await
    }

    /// Spawns and executes a child widget asynchronously, halting parent event processing
    /// until the child resolves and returns a typed output `R`.
    ///
    /// # Type Parameters
    /// - `R`: Output type returned by the sub-widget (must be `'static + Send`).
    /// - `F`: Enclosed execution closure accepting stdout buffer reference and width.
    pub async fn push_sub_widget<R, F>(
        &self,
        position: SubWidgetPosition,
        keep_rendered: bool,
        runner: F,
    ) -> io::Result<R>
    where
        R: Any + Send + 'static,
        F: for<'a> FnOnce(
                &'a mut io::BufWriter<io::Stdout>,
                usize,
            ) -> Pin<
                Box<
                    dyn std::future::Future<Output = io::Result<(Box<dyn Any + Send>, Vec<String>)>>
                        + Send
                        + 'a,
                >,
            > + Send
            + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();

        let command = Command::PushSubWidget {
            runner: Box::new(runner),
            position,
            keep_rendered,
            reply: reply_tx,
        };

        self.send_command(command);

        let raw_result = reply_rx.await.map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "SubWidget runner channel dropped",
            )
        })?;

        raw_result.downcast::<R>().map(|boxed| *boxed).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "SubWidget returned an unexpected type during downcast",
            )
        })
    }
}

impl<T: Clone + Send + Sync + 'static, E: Send + 'static> Deref for Context<T, E> {
    type Target = atoman::StateGuard<T>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T: Clone + Send + Sync + 'static, E: Send + 'static> DerefMut for Context<T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
