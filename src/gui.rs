use anyhow::Result;
use async_std::{
    channel::{unbounded, Receiver, Sender},
    stream::StreamExt,
    task,
};
use druid::{
    widget::{Align, Button, Flex, Label, TextBox},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, ExtEventSink, Handled, Lens,
    Selector, Target, Widget, WidgetExt, WindowDesc, WindowId,
};
use futures::Stream;
use log::{error, info};
use std::fmt::Debug;

#[derive(Debug)]
enum GuiEvent {
    Rename(String),
    Exit,
}

// Gui Commands
const RENAME: Selector<String> = Selector::new("rename");
const RENAMED: Selector<()> = Selector::new("renamed");

#[derive(Clone, Data, Lens)]
struct HelloState {
    name: String,
    new_name: String,
}

impl Default for HelloState {
    fn default() -> Self {
        Self {
            name: "World".into(),
            new_name: "World".into(),
        }
    }
}

struct Delegate {
    ev_snd: EvSend,
}

type EvSend = Sender<GuiEvent>;
type EvRecv = Receiver<GuiEvent>;

impl AppDelegate<HelloState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut HelloState,
        _env: &Env,
    ) -> Handled {
        dbg!(cmd);
        if let Some(name) = cmd.get(RENAME) {
            data.name = name.clone();
            Handled::Yes
        } else if let Some(()) = cmd.get(RENAMED) {
            let future = self.ev_snd.send(GuiEvent::Rename(data.new_name.clone()));
            if let Err(e) = task::block_on(future) {
                error!("{}", e);
            }
            dbg!("lol");
            Handled::Yes
        } else {
            Handled::No
        }
    }

    fn window_removed(
        &mut self,
        _id: WindowId,
        _data: &mut HelloState,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        if let Err(e) = task::block_on(self.ev_snd.send(GuiEvent::Exit)) {
            error!("{}", e);
        }
    }
}

async fn setup_gui() -> Result<(ExtEventSink, EvRecv)> {
    let (hs, hr) = unbounded();
    let (es, er) = unbounded::<GuiEvent>();

    task::spawn_blocking(|| gui_task(es, hs)).await?;

    Ok((hr.recv().await?, er))
}

fn gui_task(es: EvSend, hs: Sender<ExtEventSink>) -> Result<()> {
    let main_window = WindowDesc::new(build_root_widget)
        .title("p2ptest")
        .window_size((400.0, 400.0));

    let initial_state = HelloState::default();
    let delegate = Delegate { ev_snd: es };

    let app_launcher = AppLauncher::with_window(main_window).delegate(delegate);
    task::block_on(hs.send(app_launcher.get_external_handle()))?;
    app_launcher.launch(initial_state)?;
    dbg!("lel");
    Ok(())
}

fn build_root_widget() -> impl Widget<HelloState> {
    let label = Label::new(|data: &HelloState, _env: &Env| format!("Hello {}!", data.name));
    let textbox = TextBox::new()
        .with_placeholder("Who are we greeting?")
        .fix_width(200.)
        .lens(HelloState::new_name);
    let submit = Button::new("Submit").on_click(|ctx, _, _| ctx.submit_command(RENAMED));

    let layout = Flex::column()
        .with_child(label)
        .with_spacer(20.)
        .with_child(textbox)
        .with_spacer(20.)
        .with_child(submit);

    Align::centered(layout)
}

enum Ev<T: Debug> {
    Net(T),
    Gui(GuiEvent),
}

pub async fn run<T>(swarm: &mut T) -> Result<()>
where
    T: Stream + Unpin,
    <T as Stream>::Item: Debug,
{
    let (gui_handle, er) = setup_gui().await?;
    let mut stream = er.map(|v| Ev::Gui(v)).merge(swarm.map(|v| Ev::Net(v)));

    while let Some(ev) = stream.next().await {
        match ev {
            Ev::Net(net_event) => info!("{:?}", net_event),
            Ev::Gui(gui_event) => match dbg!(gui_event) {
                GuiEvent::Rename(name) => gui_handle.submit_command(RENAME, name, Target::Auto)?,
                GuiEvent::Exit => break,
            },
        }
    }

    Ok(())
}
