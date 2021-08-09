use crossbeam_channel::{Receiver, Sender, unbounded};
use crossterm::event::KeyEvent;
use tui::{Frame, backend::Backend, layout::Rect};

use self::{db_list::DBListController, error::ErrorController, home::HomeController, main_widget::TabsMenu, set_list::SetListController};

pub mod home;
pub mod error;
pub mod db_list;
pub mod set_list;

pub mod main_widget;

pub type ControllerDispatcher = Sender<ControllerMessage>;

pub enum ControllerMessage {
    GoHome,
    GoDB,
    GoSets,
    DBSelected(String),
}

#[derive(Copy, Clone, Debug)]
pub enum MenuItem {
    Home,
    Database,
    Sets,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Database => 1,
            MenuItem::Sets => 2,
        }
    }
}

pub trait RomstController<T: Backend> {
    fn render_view(&mut self, frame: &mut Frame<T>, area: Rect);
    fn process_key(&mut self, event: KeyEvent);
}

pub trait RomstView<T: Backend> {
    fn render_in(&mut self, frame: &mut Frame<T>, area: Rect);
}

struct ControllerData {
    db_file: Option<String>
}

impl ControllerData {
    fn new() -> Self { Self { db_file: None } }
}

pub struct ControllerManager<T: Backend> {
    tx: Sender<ControllerMessage>,
    rx: Receiver<ControllerMessage>,
    tabs_menu: TabsMenu,
    active_controller: Box<dyn RomstController<T>>,
    controller_data: ControllerData
}

impl<T: Backend> ControllerManager<T> {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        let controller_data = ControllerData::new();
        let tabs_menu = TabsMenu::new();
        Self { tx, rx, tabs_menu, active_controller: Box::new(HomeController::new()), controller_data }
    }

    pub fn update(&mut self, frame: &mut Frame<T>, area_tabs: Rect, area_contents: Rect) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                ControllerMessage::GoHome => {
                    self.active_controller = Box::new(HomeController::new());
                }
                ControllerMessage::GoDB => {
                    let sender = self.tx.clone();
                    match &self.controller_data.db_file {
                        Some(db_file) => {
                            self.active_controller = Box::new(SetListController::new(db_file));
                        }
                        None => {
                            self.active_controller = Box::new(DBListController::new(sender));
                        }
                    }
                }
                ControllerMessage::GoSets => {
                    match &self.controller_data.db_file {
                        Some(db_file) => {

                        }
                        None => {
                            self.active_controller = Box::new(ErrorController::new("Please first select a database."));
                        }
                    }
                }
                ControllerMessage::DBSelected(db_file) => {
                    self.controller_data.db_file = Some(db_file.clone());
                    self.active_controller = Box::new(SetListController::new(db_file));
                }
            }
        }

        self.tabs_menu.render_in(frame, area_tabs);
        self.active_controller.render_view(frame, area_contents);
    }

    pub fn select_menu_item(&mut self, menu_item: MenuItem) {
        let r = match menu_item {
            MenuItem::Home => {
                self.active_controller = Box::new(HomeController::new());
            }
            MenuItem::Database => {
                let sender = self.tx.clone();
                match &self.controller_data.db_file {
                    Some(db_file) => {
                        self.active_controller = Box::new(SetListController::new(db_file));
                    }
                    None => {
                        self.active_controller = Box::new(DBListController::new(sender));
                    }
                }
            }
            MenuItem::Sets => {
                match &self.controller_data.db_file {
                    Some(db_file) => {

                    }
                    None => {
                        self.active_controller = Box::new(ErrorController::new("Please first select a database."));
                    }
                }
            }
        };
        /* if let Err(e) = r {
            self.controller_manager.set_active_controller(Box::new(ErrorController::new(format!("{}", e))));
        } */
        self.tabs_menu.active_menu_item = menu_item;
    }


    pub fn get_sender(&self) -> Sender<ControllerMessage> {
        let tx = self.tx.clone(); 
        tx
    }

    pub fn process_keys_active_controller(&mut self, event: KeyEvent) {
        self.active_controller.process_key(event);
    }

    pub fn set_active_controller(&mut self, active_controller: Box<dyn RomstController<T>>) {
        self.active_controller = active_controller;
    }
}

