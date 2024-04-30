use anyhow::{Context, Result};
use directories::ProjectDirs;
use druid::WindowDesc;
use livesplit_core::{
    layout::{self, Layout, LayoutSettings},
    run::{
        parser::{composite, TimerKind},
        saver::livesplit::save_timer,
        LinkedLayout,
    },
    HotkeyConfig, HotkeySystem, Run, RunEditor, Segment, SharedTimer, Timer, TimingMethod,
};
use log::error;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, create_dir_all},
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{timer_form, LayoutData, MainState};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    splits: Splits,
    #[serde(default)]
    general: General,
    #[serde(default)]
    log: Log,
    #[serde(default)]
    window: Window,
    #[serde(default)]
    hotkeys: HotkeyConfig,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Splits {
    current: Option<PathBuf>,
    #[serde(skip)]
    can_save: bool,
    #[serde(default)]
    history: BTreeMap<Arc<str>, BTreeMap<Arc<str>, BTreeSet<Arc<Path>>>>,
}

impl Splits {
    fn add_to_history(&mut self, run: &Run) {
        if let Some(current) = &self.current {
            self.history
                .entry(run.game_name().into())
                .or_default()
                .entry(
                    run.extended_category_name(false, false, true)
                        .to_string()
                        .into(),
                )
                .or_default()
                .insert(current.as_path().into());
        }
    }

    fn remove_from_history(&mut self) {
        if let Some(current) = self.current.as_deref() {
            self.history.retain(|_, categories| {
                categories.retain(|_, paths| {
                    paths.remove(current);
                    !paths.is_empty()
                });
                !categories.is_empty()
            });
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct SplitsHistoryEntry {
    game: Box<str>,
    category: Box<str>,
    path: Box<Path>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct General {
    layout: Option<PathBuf>,
    #[serde(skip)]
    can_save_layout: bool,
    timing_method: Option<TimingMethod>,
    comparison: Option<String>,
    auto_splitter: Option<PathBuf>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Log {
    #[serde(default)]
    enable: bool,
    level: Option<log::LevelFilter>,
    #[serde(default)]
    clear: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
struct Window {
    width: f64,
    height: f64,
}

impl Default for Window {
    fn default() -> Window {
        Self {
            width: 300.0,
            height: 500.0,
        }
    }
}

static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    ProjectDirs::from("org", "LiveSplit", "LiveSplit One")
        .map(|dirs| dirs.data_local_dir().join("config.yml"))
        .unwrap_or_default()
});

impl Config {
    pub fn load() -> Self {
        Self::parse().unwrap_or_default()
    }

    fn save_config(&self) -> Option<()> {
        create_dir_all(CONFIG_PATH.parent()?).ok()?;
        self.serialize()
    }

    fn parse() -> Option<Self> {
        let buf = fs::read(CONFIG_PATH.as_path()).ok()?;
        serde_yaml::from_slice(&buf).ok()
    }

    fn serialize(&self) -> Option<()> {
        let buf = serde_yaml::to_string(self).ok()?;
        fs::write(CONFIG_PATH.as_path(), buf).ok()
    }

    pub fn splits_history(&self) -> &BTreeMap<Arc<str>, BTreeMap<Arc<str>, BTreeSet<Arc<Path>>>> {
        &self.splits.history
    }

    fn parse_run(&self) -> Option<(Run, bool)> {
        let path = self.splits.current.clone()?;
        let file = fs::read(&path).ok()?;
        let parsed_run = composite::parse(&file, Some(&path)).ok()?;
        let run = parsed_run.run;
        let can_save = parsed_run.kind == TimerKind::LiveSplit;
        Some((run, can_save))
    }

    pub fn parse_run_or_default(&mut self) -> Run {
        match self.parse_run() {
            Some((run, can_save)) => {
                self.splits.can_save = can_save;
                run
            }
            None => {
                self.splits.can_save = false;
                default_run()
            }
        }
    }

    pub fn is_game_time(&self) -> bool {
        self.general.timing_method == Some(TimingMethod::GameTime)
    }

    fn parse_layout_with_path(path: &Path) -> Result<(Layout, bool)> {
        let file = fs::read_to_string(path).context("Failed reading the file.")?;
        match LayoutSettings::from_json(Cursor::new(&file)) {
            Ok(settings) => return Ok((Layout::from_settings(settings), true)),
            Err(err) => error!("Failed to parse layout as *.ls1l: {err}"),
        }
        layout::parser::parse(&file)
            .context("Failed parsing the layout.")
            .map(|layout| (layout, false))
    }

    fn parse_layout(&mut self, timer: &Timer) -> Option<Layout> {
        if let Some(linked_layout) = timer.run().linked_layout() {
            match linked_layout {
                LinkedLayout::Default => {
                    self.general.can_save_layout = false;
                    self.general.layout = None;
                    self.save_config();
                    return None;
                }
                LinkedLayout::Path(path) => {
                    if let Ok((layout, can_save)) = Self::parse_layout_with_path(Path::new(path)) {
                        self.general.can_save_layout = can_save;
                        self.general.layout = Some(path.into());
                        self.save_config();
                        return Some(layout);
                    }
                }
            }
        }

        let (layout, can_save) =
            Self::parse_layout_with_path(self.general.layout.as_deref()?).ok()?;
        self.general.can_save_layout = can_save;
        Some(layout)
    }

    pub fn parse_layout_or_default(&mut self, timer: &Timer) -> Layout {
        self.parse_layout(timer)
            .unwrap_or_else(Layout::default_layout)
    }

    // TODO: Just directly construct the HotkeySystem from the config.
    pub fn configure_hotkeys(&self, hotkeys: &mut HotkeySystem) {
        hotkeys.set_config(self.hotkeys).ok();
    }

    pub fn configure_timer(&self, timer: &mut Timer) {
        if self.is_game_time() {
            timer.set_current_timing_method(TimingMethod::GameTime);
        }
        if let Some(comparison) = &self.general.comparison {
            timer.set_current_comparison(comparison.as_str()).ok();
        }
    }

    pub fn set_hotkeys(&mut self, hotkeys: HotkeyConfig) {
        self.hotkeys = hotkeys;
        self.save_config();
    }

    pub fn new_splits(&mut self, timer: &mut Timer) {
        timer.set_run(default_run()).map_err(drop).unwrap();
        self.splits.can_save = false;
        self.splits.current = None;
        self.save_config();
    }

    pub fn open_splits(
        &mut self,
        shared_timer: &SharedTimer,
        layout_data: &mut LayoutData,
        #[cfg(feature = "auto-splitting")]
        auto_splitter: &livesplit_core::auto_splitting::Runtime,
        path: PathBuf,
    ) -> Result<()> {
        {
            let timer = &mut shared_timer.write().unwrap();
            let file = fs::read(&path).context("Failed reading the file.")?;
            let run = composite::parse(&file, Some(&path)).context("Failed parsing the file.")?;
            timer.set_run(run.run).ok().context(
                "The splits can't be used with the timer because they don't contain a single segment.",
            )?;

            self.splits.can_save = run.kind == TimerKind::LiveSplit;
            self.splits.current = Some(path);
            self.splits.add_to_history(timer.run());

            self.save_config();

            if let Some(linked_layout) = timer.run().linked_layout() {
                match linked_layout {
                    LinkedLayout::Default => self.new_layout(None, layout_data),
                    LinkedLayout::Path(path) => {
                        let _ = self.open_layout(None, layout_data, Path::new(path));
                    }
                }
            }
        }

        #[cfg(feature = "auto-splitting")]
        auto_splitter.reload(shared_timer.clone())?;

        Ok(())
    }

    pub fn can_directly_save_splits(&self) -> bool {
        self.splits.current.is_some() && self.splits.can_save
    }

    pub fn save_splits(
        &mut self,
        timer: &mut Timer,
        #[cfg(feature = "auto-splitting")]
        runtime: &livesplit_core::auto_splitting::Runtime,
    ) -> Result<()> {
        if let Some(path) = &self.splits.current {
            #[cfg(feature = "auto-splitting")]
            runtime.update_timer_auto_splitter_settings(timer);
            let mut buf = String::new();
            save_timer(timer, &mut buf).context("Failed saving the splits.")?;
            fs::write(path, &buf).context("Failed writing the file.")?;
            timer.mark_as_unmodified();

            self.splits.remove_from_history();
            self.splits.add_to_history(timer.run());

            self.save_config();
        }
        Ok(())
    }

    pub fn save_splits_as(
        &mut self,
        timer: &mut Timer,
        #[cfg(feature = "auto-splitting")]
        runtime: &livesplit_core::auto_splitting::Runtime,
        path: PathBuf,
    ) -> Result<()> {
        #[cfg(feature = "auto-splitting")]
        runtime.update_timer_auto_splitter_settings(timer);
        let mut buf = String::new();
        save_timer(timer, &mut buf).context("Failed saving the splits.")?;
        fs::write(&path, &buf).context("Failed writing the file.")?;
        timer.mark_as_unmodified();

        if !self.splits.can_save {
            self.splits.remove_from_history();
        }
        self.splits.can_save = true;
        self.splits.current = Some(path);
        self.splits.add_to_history(timer.run());

        self.save_config();
        Ok(())
    }

    pub fn link_layout(&self, run_editor: &mut RunEditor) {
        run_editor.set_linked_layout(Some(
            match &self.general.layout {
                Some(path) => path.to_str().map(|p| LinkedLayout::Path(p.to_owned())),
                None => None,
            }
            .unwrap_or(LinkedLayout::Default),
        ));
    }

    pub fn new_layout(&mut self, timer: Option<&mut Timer>, layout_data: &mut LayoutData) {
        self.general.can_save_layout = false;
        self.general.layout = None;
        layout_data.layout = Layout::default_layout();
        layout_data.is_modified = false;

        if let Some(timer) = timer {
            timer.layout_path_changed(None::<&str>);
        }

        self.save_config();
    }

    pub fn open_layout(
        &mut self,
        timer: Option<&mut Timer>,
        layout_data: &mut LayoutData,
        path: &Path,
    ) -> Result<()> {
        let (layout, can_save) = Self::parse_layout_with_path(path)?;
        self.general.can_save_layout = can_save;
        self.general.layout = Some(path.into());
        layout_data.layout = layout;
        layout_data.is_modified = false;

        if let Some(timer) = timer {
            timer.layout_path_changed(path.to_str());
        }

        self.save_config();
        Ok(())
    }

    pub fn save_layout(&self, settings: LayoutSettings) -> Result<()> {
        if let Some(path) = &self.general.layout {
            let mut buf = Vec::new();
            settings
                .write_json(&mut buf)
                .context("Failed saving the layout.")?;
            fs::write(path, &buf).context("Failed writing the file.")?;
        }
        Ok(())
    }

    pub fn save_layout_as(
        &mut self,
        timer: &mut Timer,
        settings: LayoutSettings,
        path: PathBuf,
    ) -> Result<()> {
        let mut buf = Vec::new();
        settings
            .write_json(&mut buf)
            .context("Failed saving the layout.")?;
        fs::write(&path, &buf).context("Failed writing the file.")?;

        timer.layout_path_changed(path.to_str());

        self.general.can_save_layout = true;
        self.general.layout = Some(path);
        self.save_config();
        Ok(())
    }

    pub fn can_directly_save_layout(&self) -> bool {
        self.general.layout.is_some() && self.general.can_save_layout
    }

    pub fn open_auto_splitter(
        &mut self,
        #[cfg(feature = "auto-splitting")]
        shared_timer: &SharedTimer,
        #[cfg(feature = "auto-splitting")]
        runtime: &livesplit_core::auto_splitting::Runtime,
        path: &Path,
    ) -> Result<()> {
        self.general.auto_splitter = Some(path.into());
        self.save_config();
        #[cfg(feature = "auto-splitting")]
        runtime.unload()?;
        #[cfg(feature = "auto-splitting")]
        runtime.load(path.into(), shared_timer.clone())?;
        Ok(())
    }

    pub fn set_comparison(&mut self, comparison: String) {
        self.general.comparison = Some(comparison);
        self.save_config();
    }

    pub fn set_timing_method(&mut self, timing_method: TimingMethod) {
        self.general.timing_method = Some(timing_method);
        self.save_config();
    }

    pub fn setup_logging(&self) -> Option<()> {
        if self.log.enable {
            let config_folder = CONFIG_PATH.parent()?;
            create_dir_all(config_folder).ok()?;

            let log_file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(!self.log.clear)
                .truncate(self.log.clear)
                .open(config_folder.join("log.txt"))
                .ok()?;

            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "{}[{}][{}] {}",
                        chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                        record.target(),
                        record.level(),
                        message
                    ))
                })
                .level(self.log.level.unwrap_or(log::LevelFilter::Warn))
                .chain(log_file)
                .apply()
                .ok()?;

            #[cfg(not(debug_assertions))]
            {
                std::panic::set_hook(Box::new(|panic_info| {
                    log::error!(target: "PANIC", "{}\n{:?}", panic_info, backtrace::Backtrace::new());
                }));
            }
        }
        Some(())
    }

    pub fn build_window(&self) -> WindowDesc<MainState> {
        WindowDesc::new(timer_form::root_widget())
            .title("LiveSplit One")
            .with_min_size((50.0, 50.0))
            .window_size((self.window.width, self.window.height))
            .show_titlebar(false)
            .transparent(true)
            .set_always_on_top(true)
    }

    #[cfg(feature = "auto-splitting")]
    pub fn maybe_load_auto_splitter(&self, runtime: &livesplit_core::auto_splitting::Runtime, timer: SharedTimer) {
        if let Some(auto_splitter) = &self.general.auto_splitter {
            if let Err(e) = runtime.load(auto_splitter.clone(), timer) {
                // TODO: Error chain
                log::error!("Auto Splitter failed to load: {}", e);
            }
        }
    }
}

fn default_run() -> Run {
    let mut run = Run::new();
    run.push_segment(Segment::new("Time"));
    run
}

pub fn show_error(error: anyhow::Error) {
    // this MessageDialog is for displaying errors, 
    // so I guess it's fine if it crashes? if it was going to crash anyway?
    let _ = native_dialog::MessageDialog::new()
        .set_type(native_dialog::MessageType::Error)
        .set_title("Error")
        .set_text(&format!("{error:?}"))
        .show_alert();
}

pub fn or_show_error(result: Result<()>) {
    if let Err(e) = result {
        show_error(e);
    }
}
