//! Native library notifications and background discovery, owned by the frontend.

use crate::game_library::{self, GameLibrary, GameManifest, LibraryGame, SaveCatalog};
use crate::library_catalog::{self, CatalogRead};
use async_channel::{Receiver, Sender};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const REFRESH_DELAY: Duration = Duration::from_millis(80);
const WRITE_PROBE_PREFIX: &str = ".library-write-check-";

/// The watcher sends hints to one selected root. Each kind occupies one slot.
#[derive(Debug, Clone)]
pub struct LibraryNotifications {
    changed: Sender<()>,
    rescan: Sender<()>,
    error: Sender<String>,
}

impl LibraryNotifications {
    /// Returns whether the caller needs to wake the frontend.
    pub fn changed(&self) -> bool {
        self.changed.try_send(()).is_ok()
    }

    /// A root replacement or missed events need a new watch and a full scan.
    pub fn rescan(&self) -> bool {
        self.rescan.try_send(()).is_ok()
    }

    pub fn failed(&self, error: String) -> bool {
        let error_pending = self.error.try_send(error).is_ok();
        self.rescan() || error_pending
    }
}

#[derive(Debug)]
pub struct LibraryRefreshSchedule {
    sender: LibraryNotifications,
    changed: Receiver<()>,
    rescan: Receiver<()>,
    error: Receiver<String>,
    deadline: Option<Instant>,
    needs_rescan: bool,
}

impl Default for LibraryRefreshSchedule {
    fn default() -> Self {
        let (changed_tx, changed) = async_channel::bounded(1);
        let (rescan_tx, rescan) = async_channel::bounded(1);
        let (error_tx, error) = async_channel::bounded(1);
        Self {
            sender: LibraryNotifications {
                changed: changed_tx,
                rescan: rescan_tx,
                error: error_tx,
            },
            changed,
            rescan,
            error,
            deadline: None,
            needs_rescan: false,
        }
    }
}

impl LibraryRefreshSchedule {
    pub fn notifications(&self) -> LibraryNotifications {
        self.sender.clone()
    }

    /// Collects bounded hints. The first event sets the deadline even in a sustained burst.
    pub fn receive(&mut self, now: Instant) {
        let changed = self.changed.try_recv().is_ok();
        let rescan = self.rescan.try_recv().is_ok();
        if changed || rescan {
            self.deadline.get_or_insert(now + REFRESH_DELAY);
        }
        self.needs_rescan |= rescan;
    }

    pub fn take_error(&self) -> Option<String> {
        self.error.try_recv().ok()
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn needs_rescan(&self) -> bool {
        self.needs_rescan
    }

    /// At the deadline, returns whether the next discovery also needs to rebind.
    pub fn take_ready(&mut self, now: Instant) -> Option<bool> {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.deadline = None;
            Some(std::mem::take(&mut self.needs_rescan))
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct Watch {
    watcher: Option<RecommendedWatcher>,
    warning: Option<String>,
    write_error: Option<String>,
}

impl Watch {
    fn attach(root: &Path, notifications: LibraryNotifications, ctx: egui::Context) -> Self {
        use notify::event::{AccessKind, AccessMode, MetadataKind, ModifyKind};
        use notify::EventKind;

        let write_error = tempfile::Builder::new()
            .prefix(WRITE_PROBE_PREFIX)
            .tempfile_in(root)
            .and_then(tempfile::NamedTempFile::close)
            .err()
            .map(|error| error.to_string());
        if matches!(RecommendedWatcher::kind(), notify::WatcherKind::PollWatcher) {
            return Self {
                watcher: None,
                warning: Some("This platform has no supported notification backend".into()),
                write_error,
            };
        }
        let watched = root.to_path_buf();
        let callback = move |result: notify::Result<notify::Event>| {
            let wake = match result {
                Err(error) => notifications.failed(error.to_string()),
                Ok(event) if event.need_rescan() => notifications.rescan(),
                Ok(event) if matches!(event.kind, EventKind::Access(kind) if kind != AccessKind::Close(AccessMode::Write)) => {
                    false
                }
                Ok(event)
                    if matches!(
                        event.kind,
                        EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime))
                    ) =>
                {
                    false
                }
                Ok(event) => {
                    let root_changed = event.paths.iter().any(|p| p == &watched)
                        && matches!(
                            event.kind,
                            EventKind::Any
                                | EventKind::Create(_)
                                | EventKind::Remove(_)
                                | EventKind::Modify(ModifyKind::Name(_) | ModifyKind::Metadata(_))
                        );
                    if root_changed {
                        notifications.rescan()
                    } else if event.paths.is_empty()
                        || event.paths.iter().any(|p| {
                            p.starts_with(&watched)
                                && !p.strip_prefix(&watched).ok().is_some_and(|relative| {
                                    relative.components().count() == 1
                                        && relative
                                            .as_os_str()
                                            .to_string_lossy()
                                            .starts_with(WRITE_PROBE_PREFIX)
                                })
                        })
                    {
                        notifications.changed()
                    } else {
                        false
                    }
                }
            };
            if wake {
                ctx.request_repaint();
            }
        };
        match notify::recommended_watcher(callback) {
            Ok(mut watcher) => {
                // A parent watch observes replacement even if the old root watch is lost.
                let parent_error = root
                    .parent()
                    .and_then(|parent| watcher.watch(parent, RecursiveMode::NonRecursive).err());
                let root_error = watcher.watch(root, RecursiveMode::Recursive).err();
                Self {
                    watcher: Some(watcher),
                    warning: root_error.or(parent_error).map(|error| error.to_string()),
                    write_error,
                }
            }
            Err(error) => Self {
                watcher: None,
                warning: Some(error.to_string()),
                write_error,
            },
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            blocking::unblock(move || drop(watcher)).detach();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SaveReadTarget {
    directory: PathBuf,
    manifest: GameManifest,
}

impl From<&LibraryGame> for SaveReadTarget {
    fn from(game: &LibraryGame) -> Self {
        Self {
            directory: game.directory.clone(),
            manifest: game.manifest.clone(),
        }
    }
}

impl SaveReadTarget {
    fn matches(&self, game: &LibraryGame) -> bool {
        self.directory == game.directory && self.manifest == game.manifest
    }
}

#[derive(Debug)]
struct Discovery {
    library: GameLibrary,
    catalog: CatalogRead,
    canonical_root: Option<PathBuf>,
    selected: Option<(String, Result<SaveCatalog, String>)>,
    active: Option<(SaveReadTarget, Result<SaveCatalog, String>)>,
}

impl Discovery {
    fn read(
        root: &Path,
        selected_id: Option<String>,
        active: Option<SaveReadTarget>,
    ) -> Result<Self, String> {
        let library = GameLibrary::discover(root)?;
        let selected = selected_id.and_then(|id| {
            library
                .games
                .iter()
                .find(|game| game.manifest.id == id)
                .map(|game| (id, game_library::saves(game)))
        });
        let active = active.map(|game| {
            let saves = game_library::saves_in(&game.directory, &game.manifest);
            (game, saves)
        });
        Ok(Self {
            library,
            catalog: library_catalog::read(root, ""),
            canonical_root: root.canonicalize().ok(),
            selected,
            active,
        })
    }
}

#[derive(Debug)]
struct Scan {
    revision: u64,
    watch: Option<Watch>,
    discovery: Result<Discovery, String>,
}

#[derive(Debug)]
pub(crate) struct NativeLibrary {
    setting: PathBuf,
    pub root: PathBuf,
    canonical_root: Option<PathBuf>,
    schedule: LibraryRefreshSchedule,
    watch: Option<Watch>,
    job: Option<Receiver<Scan>>,
    pub read_error: Option<String>,
    runtime_error: Option<String>,
    revision: u64,
}

impl NativeLibrary {
    fn new(setting: PathBuf) -> Self {
        let (root, read_error) = match std::path::absolute(&setting) {
            Ok(root) => (root, None),
            Err(error) => (setting.clone(), Some(error.to_string())),
        };
        Self {
            setting,
            root,
            canonical_root: None,
            schedule: LibraryRefreshSchedule::default(),
            watch: None,
            job: None,
            read_error,
            runtime_error: None,
            revision: 0,
        }
    }

    pub(crate) fn warning(&self) -> Option<&str> {
        self.runtime_error
            .as_deref()
            .or_else(|| self.watch.as_ref()?.warning.as_deref())
    }

    pub(crate) fn reconciling(&self) -> bool {
        self.schedule.needs_rescan() || (self.watch.is_none() && self.job.is_some())
    }

    pub(crate) fn writable(&self) -> bool {
        self.read_error.is_none()
            && self
                .watch
                .as_ref()
                .is_some_and(|watch| watch.write_error.is_none())
    }

    pub(crate) fn write_error(&self) -> Option<&str> {
        self.watch.as_ref()?.write_error.as_deref()
    }

    pub(crate) fn owns_game(&self, game: &LibraryGame) -> bool {
        self.owns_game_directory(&game.directory)
    }

    pub(crate) fn owns_game_directory(&self, directory: &Path) -> bool {
        directory
            .parent()
            .and_then(|parent| std::path::absolute(parent).ok())
            .is_some_and(|parent| {
                parent == self.root || self.canonical_root.as_ref() == Some(&parent)
            })
    }

    pub(crate) fn changed_locally(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        let _ = self.schedule.notifications().changed();
    }

    fn scan(
        &mut self,
        ctx: &egui::Context,
        rebind: bool,
        selected: Option<String>,
        active: Option<&LibraryGame>,
    ) {
        let (sender, receiver) = async_channel::bounded(1);
        self.job = Some(receiver);
        let root = self.root.clone();
        let revision = self.revision;
        let active = active
            .filter(|game| self.owns_game(game))
            .map(SaveReadTarget::from);
        let ctx = ctx.clone();
        let notifications = self.schedule.notifications();
        let old_watch = rebind.then(|| self.watch.take()).flatten();
        blocking::unblock(move || {
            if let Some(mut watch) = old_watch {
                drop(watch.watcher.take());
            }
            if sender.is_closed() {
                return;
            }
            let watch = rebind.then(|| Watch::attach(&root, notifications, ctx.clone()));
            let discovery = Discovery::read(&root, selected, active);
            if sender
                .try_send(Scan {
                    revision,
                    watch,
                    discovery,
                })
                .is_ok()
            {
                ctx.request_repaint();
            }
        })
        .detach();
    }

    fn poll(
        &mut self,
        ctx: &egui::Context,
        selected: Option<String>,
        active: Option<&LibraryGame>,
    ) -> Option<Result<Discovery, String>> {
        let now = Instant::now();
        self.schedule.receive(now);
        let error_now = self.schedule.take_error();
        if let Some(error) = &error_now {
            self.runtime_error = Some(error.clone());
        }

        let mut completed = None;
        if let Some(job) = &self.job {
            match job.try_recv() {
                Ok(scan) => {
                    self.job = None;
                    if let Some(watch) = scan.watch {
                        self.watch = Some(watch);
                    }
                    if !self.schedule.needs_rescan() && scan.revision == self.revision {
                        if scan.discovery.is_ok() && error_now.is_none() {
                            self.runtime_error = None;
                        }
                        completed = Some(scan.discovery);
                    }
                }
                Err(async_channel::TryRecvError::Empty) => {}
                Err(async_channel::TryRecvError::Closed) => {
                    self.job = None;
                    completed = Some(Err("Background discovery stopped unexpectedly".into()));
                }
            }
        }
        if self.job.is_none() {
            if let Some(rebind) = self.schedule.take_ready(now) {
                self.scan(ctx, rebind, selected, active);
            } else if let Some(deadline) = self.schedule.deadline() {
                ctx.request_repaint_after(deadline.saturating_duration_since(now));
            }
        }
        completed
    }
}

impl crate::player::PlayerState {
    pub(crate) fn clear_library_listing(&mut self) {
        self.library = GameLibrary::default();
        self.selected_game = None;
        self.set_selected_saves(Ok(SaveCatalog::default()));
        self.library_editor = None;
        self.missing_configuration = None;
    }

    pub(crate) fn selected_entry(&self) -> Option<&crate::game_library::LibraryGame> {
        let id = self.selected_game.as_ref()?;
        self.library
            .games
            .iter()
            .find(|game| &game.manifest.id == id)
    }

    pub(crate) fn set_selected_saves(&mut self, result: Result<SaveCatalog, String>) {
        self.save_catalog.times.changed();
        match result {
            Ok(saves) => {
                self.selected_saves = saves.entries;
                self.selected_save_error = saves.warning;
            }
            Err(error) => {
                self.selected_saves.clear();
                self.selected_save_error = Some(error);
            }
        }
    }

    pub(crate) fn set_session_saves(&mut self, result: Result<SaveCatalog, String>) {
        self.save_catalog.times.changed();
        match result {
            Ok(saves) => {
                self.saves = saves.entries;
                self.session_save_error = saves.warning;
            }
            Err(error) => {
                self.saves.clear();
                self.session_save_error = Some(error);
            }
        }
    }

    pub(crate) fn refresh_selected_saves(&mut self) {
        let saves = self
            .selected_entry()
            .map_or_else(|| Ok(SaveCatalog::default()), game_library::saves);
        self.set_selected_saves(saves);
    }
}

impl crate::EmulatorApp {
    pub(crate) fn refresh_player_saves(&mut self) {
        if let Some(game) = &self.player.active_game {
            let saves = game_library::saves(game);
            self.player.set_session_saves(saves);
        }
    }

    pub(crate) fn poll_library(&mut self, ctx: &egui::Context) {
        if self.closing {
            self.player.library_sync = None;
            return;
        }
        let setting = self.player.library_root.as_ref();
        let watched = self.player.library_sync.as_ref().map(|sync| &sync.setting);
        if watched != setting {
            let setting = setting.cloned();
            if self.player.library_sync.take().is_some() {
                self.player.clear_library_listing();
            }
            if let Some(setting) = setting {
                let mut sync = NativeLibrary::new(setting);
                if sync.read_error.is_none() {
                    sync.scan(
                        ctx,
                        true,
                        self.player.selected_game.clone(),
                        self.player.active_game.as_ref(),
                    );
                }
                self.player.library_sync = Some(sync);
            }
            return;
        }
        let selection = self.player.selected_game.clone();
        let active = self.player.active_game.as_ref();
        let Some(sync) = &mut self.player.library_sync else {
            return;
        };
        match sync.poll(ctx, selection, active) {
            Some(Ok(discovered)) => {
                sync.read_error = None;
                sync.canonical_root = discovered.canonical_root;
                self.player.library = discovered.library;
                self.reconcile_library_catalog(discovered.catalog, ctx);
                match discovered.selected {
                    Some((id, saves)) if self.player.selected_game.as_ref() == Some(&id) => {
                        self.player.set_selected_saves(saves);
                    }
                    _ if self.player.selected_entry().is_some() => {
                        // The player picked another entry while discovery was in flight.
                        if let Some(sync) = &self.player.library_sync {
                            if sync.schedule.notifications().changed() {
                                ctx.request_repaint();
                            }
                        }
                    }
                    _ => {
                        self.player.selected_saves.clear();
                        self.player.selected_save_error = None;
                    }
                }
                if let Some((target, saves)) = discovered.active {
                    if self
                        .player
                        .active_game
                        .as_ref()
                        .is_some_and(|game| target.matches(game))
                    {
                        self.player.set_session_saves(saves);
                    }
                }
            }
            Some(Err(error)) => {
                sync.read_error = Some(error);
                self.player.library = GameLibrary::default();
                self.player.selected_saves.clear();
                self.player.selected_save_error = None;
            }
            None => {}
        }
    }
}
