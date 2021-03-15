use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::event::KeyEvent;
use tui::{Frame, backend::Backend, layout::Rect};

use self::home_widget::HomeWidget;

pub mod tabs_widget;
pub mod home_widget;
pub mod error_widget;
pub mod list_db_widget;
pub mod list_set_widget;

pub type WidgetDispatcher<'a, T> = Sender<ViewMessage<'a, T>>;

pub trait RomstWidget<'a, T: Backend + 'a> {
    fn render_in(&mut self, frame: &mut Frame<T>, area: Rect);
    fn process_key(&mut self, event: KeyEvent);
    // Sender for opening new window
    fn set_sender(&mut self, sender: WidgetDispatcher<'a, T>);
}

pub enum ViewMessage<'a, T: Backend> {
    NewView(Box<dyn RomstWidget<'a, T>>),
}

pub struct ViewManager<'a, T: Backend> {
    tx: Sender<ViewMessage<'a, T>>,
    rx: Receiver<ViewMessage<'a, T>>,
    active_widget: Box<dyn RomstWidget<'a, T>>
}

impl<'a, T: Backend> ViewManager<'a, T> {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx, active_widget: Box::new(HomeWidget::new()) }
    }

    pub fn update(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                ViewMessage::NewView(view) => {
                    self.set_active_widget(view);
                }
            }
        }
    }

    pub fn get_sender(&self) -> Sender<ViewMessage<'a, T>> {
        let tx = self.tx.clone(); 
        tx
    }

    pub fn render_active_widget(&mut self, frame: &mut Frame<T>, area: Rect) {
        self.active_widget.render_in(frame, area);
    }

    pub fn process_keys_active_widget(&mut self, event: KeyEvent) {
        self.active_widget.process_key(event);
    }

    pub fn set_active_widget(&mut self, mut view: Box<dyn RomstWidget<'a, T>>) {
        view.as_mut().set_sender(self.tx.clone());
        self.active_widget = view;
    }
}
