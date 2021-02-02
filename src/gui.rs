use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver, Sender};
use druid::{
    widget::{Align, Button, Flex, Label, TextBox},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, ExtEventSink, Handled, Lens,
    Selector, Target, Widget, WidgetExt, WindowDesc, WindowId,
};
use futures::{Stream, StreamExt, ready};
use log::info;
use std::{fmt::Debug, task::{Context, Poll}, thread::spawn};

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
            // need a clean way to handle errors (env? state?)
            self.ev_snd.send(GuiEvent::Rename(data.new_name.clone())).unwrap();
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
        self.ev_snd.send(GuiEvent::Exit).unwrap();
    }
}

fn setup_gui() -> Result<(ExtEventSink, EvRecv)> {
    let (hs, hr) = unbounded();
    let (es, er) = unbounded::<GuiEvent>();

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
    let submit = Button::new("Submit").on_click(|ctx, _, _| {
        ctx.submit_command(RENAMED)
    });

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

    'main: loop {
        if let Ok(gui_event) = er.try_recv() {
            match dbg!(gui_event) {
                GuiEvent::Rename(name) => {
                    gui_handle.submit_command(RENAME, name, Target::Auto)?
                },
                GuiEvent::Exit => break 'main,
            }
        }

        let net_event = ready!(swarm.poll_next_unpin(cx));
        info!("{:?}", net_event);
    }

    Ok(())
}
