#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

use clap::Parser;
use druid::{Data, Lens, WindowId};
use livesplit_core::{
    layout::LayoutState, settings::ImageCache, HotkeySystem, Layout, SharedTimer, Timer,
};
use mimalloc::MiMalloc;
use once_cell::sync::Lazy;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use crate::config::Config;

mod cli;
mod color_button;
mod combo_box;
mod config;
mod consts;
mod formatter_scope;
mod hotkey_button;
mod layout_editor;
mod map_scope;
mod run_editor;
mod settings_editor;
mod settings_table;
mod timer_form;

mod software_renderer;
// mod piet_renderer;

static HOTKEY_SYSTEM: RwLock<Option<HotkeySystem<SharedTimer>>> = RwLock::new(None);
static FONT_FAMILIES: Lazy<Arc<[Arc<str>]>> = Lazy::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut families = db
        .faces()
        .filter_map(|face| Some(face.families.first()?.0.as_str().into()))
        .collect::<Vec<_>>();

    families.sort_unstable();
    families.dedup();

    families.into()
});

#[derive(Clone, Data, Lens)]
pub struct MainState {
    #[data(ignore)]
    timer: SharedTimer,
    #[data(ignore)]
    layout_data: Rc<RefCell<LayoutData>>,
    #[data(ignore)]
    #[cfg(feature = "auto-splitting")]
    auto_splitter: Rc<livesplit_core::auto_splitting::Runtime<SharedTimer>>,
    #[data(ignore)]
    config: Rc<RefCell<Config>>,
    run_editor: Option<OpenWindow<run_editor::State>>,
    layout_editor: Option<OpenWindow<layout_editor::State>>,
    settings_editor: Option<OpenWindow<settings_editor::State>>,
    image_cache: Rc<RefCell<ImageCache>>,
    mouse_pass_through: bool,
}

pub struct LayoutData {
    layout: Layout,
    layout_state: LayoutState,
    is_modified: bool,
}

#[derive(Clone)]
struct OpenWindow<T> {
    id: WindowId,
    state: T,
}

impl<T: Data> Data for OpenWindow<T> {
    fn same(&self, other: &Self) -> bool {
        self.id == other.id && self.state.same(&other.state)
    }
}

impl MainState {
    fn new(mut config: Config) -> Self {
        config.setup_logging();

        let run = config.parse_run_or_default();
        let mut timer = Timer::new(run).unwrap();
        config.configure_timer(&mut timer);

        let layout = config.parse_layout_or_default(&timer);

        let timer = timer.into_shared();
        let hotkey_system = config.configure_hotkeys(timer.clone());
        *HOTKEY_SYSTEM.write().unwrap() = Some(hotkey_system);

        #[cfg(feature = "auto-splitting")]
        let auto_splitter = livesplit_core::auto_splitting::Runtime::new();
        #[cfg(feature = "auto-splitting")]
        config.maybe_load_auto_splitter(&auto_splitter, timer.clone());

        Self {
            timer,
            #[cfg(feature = "auto-splitting")]
            auto_splitter: Rc::new(auto_splitter),
            layout_data: Rc::new(RefCell::new(LayoutData {
                layout,
                layout_state: LayoutState::default(),
                is_modified: false,
            })),
            config: Rc::new(RefCell::new(config)),
            run_editor: None,
            layout_editor: None,
            settings_editor: None,
            image_cache: Rc::new(RefCell::new(ImageCache::new())),
            mouse_pass_through: false,
        }
    }
}

struct RunEditorLens;

impl Lens<MainState, run_editor::State> for RunEditorLens {
    fn with<V, F: FnOnce(&run_editor::State) -> V>(&self, data: &MainState, f: F) -> V {
        f(&data.run_editor.as_ref().unwrap().state)
    }

    fn with_mut<V, F: FnOnce(&mut run_editor::State) -> V>(&self, data: &mut MainState, f: F) -> V {
        f(&mut data.run_editor.as_mut().unwrap().state)
    }
}

struct LayoutEditorLens;

impl Lens<MainState, layout_editor::State> for LayoutEditorLens {
    fn with<V, F: FnOnce(&layout_editor::State) -> V>(&self, data: &MainState, f: F) -> V {
        f(&data.layout_editor.as_ref().unwrap().state)
    }

    fn with_mut<V, F: FnOnce(&mut layout_editor::State) -> V>(
        &self,
        data: &mut MainState,
        f: F,
    ) -> V {
        f(&mut data.layout_editor.as_mut().unwrap().state)
    }
}

struct SettingsEditorLens;

impl Lens<MainState, settings_editor::State> for SettingsEditorLens {
    fn with<V, F: FnOnce(&settings_editor::State) -> V>(&self, data: &MainState, f: F) -> V {
        f(&data.settings_editor.as_ref().unwrap().state)
    }

    fn with_mut<V, F: FnOnce(&mut settings_editor::State) -> V>(
        &self,
        data: &mut MainState,
        f: F,
    ) -> V {
        f(&mut data.settings_editor.as_mut().unwrap().state)
    }
}

fn main() {
    let cli = cli::Cli::parse();
    let config = Config::load(cli.split_file);
    let window = config.build_window();
    timer_form::launch(MainState::new(config), window);
}
