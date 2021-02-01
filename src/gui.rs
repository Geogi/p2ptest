use anyhow::Result;
use druid::{
    widget::{Align, Button, Flex, Label, TextBox},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, ExtEventSink, Handled, Lens,
    Selector, Target, ValueType, Widget, WidgetExt, WindowDesc,
};
use futures::{Stream, StreamExt};
use std::{
    fmt::Debug,
    sync::mpsc::{channel, Receiver, Sender},
    thread::spawn,
};

enum NetEvent {
    Rename(String),
}

// Gui Commands
const RENAME: Selector<String> = Selector::new("rename");
const RENAMED: Selector<String> = Selector::new("renamed");

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

type EvSend = Sender<NetEvent>;
type EvRecv = Receiver<NetEvent>;

impl AppDelegate<HelloState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut HelloState,
        _env: &Env,
    ) -> Handled {
        if let Some(name) = cmd.get(RENAME) {
            data.name = name.clone();
            Handled::Yes
        } else if let Some(name) = cmd.get(RENAMED) {
            // need a clean way to handle errors (env? state?)
            let _ = self.ev_snd.send(NetEvent::Rename(name.clone()));
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn setup_gui() -> Result<(ExtEventSink, EvRecv)> {
    let (hs, hr) = channel();
    let (es, er) = channel::<NetEvent>();

    spawn(move || -> Result<()> {
        let main_window = WindowDesc::new(build_root_widget)
            .title("p2ptest")
            .window_size((400.0, 400.0));

        let initial_state = HelloState::default();
        let delegate = Delegate { ev_snd: es };

        let app_launcher = AppLauncher::with_window(main_window).delegate(delegate);
        hs.send(app_launcher.get_external_handle())?;
        app_launcher.launch(initial_state)?;
        Ok(())
    });

    Ok((hr.recv()?, er))
}

fn build_root_widget() -> impl Widget<HelloState> {
    let label = Label::new(|data: &HelloState, _env: &Env| format!("Hello {}!", data.name));
    let textbox = TextBox::new()
        .with_placeholder("Who are we greeting?")
        .fix_width(200.)
        .lens(HelloState::new_name);
    let submit = Button::new("Submit").on_click(|_, _, _| {});

    let layout = Flex::column()
        .with_child(label)
        .with_spacer(20.)
        .with_child(textbox)
        .with_spacer(20.)
        .with_child(submit);

    Align::centered(layout)
}

pub async fn run<T>(swarm: &mut T) -> Result<()>
where
    T: Stream + Unpin,
    <T as Stream>::Item: Debug,
{
    let (gui_handle, er) = setup_gui()?;

    loop {
        while let Ok(gui_event) = er.try_recv() {
            match gui_event {
                NetEvent::Rename(name) => gui_handle.submit_command(RENAME, name, Target::Auto)?,
            }
        }
        while let Some(net_event) = swarm.next().await {
            println!("{:?}", net_event);
        }
    }
}
