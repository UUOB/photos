//! Photos' window on Linux: GTK 4 and Libadwaita through Relm4, on Photos'
//! core (`core/`) and UUOB's SDK. The connect screen is UUOB's kit
//! (`uuob/kit/linux`), the same in every app.

use std::collections::HashMap;
use std::path::PathBuf;

use photos_core as core;
use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, RelmApp, adw, gtk, gtk::gdk, gtk::gdk_pixbuf, gtk::gio, gtk::glib};
use serde_json::{Value, json};
use uuob_client::{App, Client, Event};
use uuob_kit_gtk::{AppInfo, Gate};

const FEED: &str = "https://raw.githubusercontent.com/jachymri/uuob-releases/main/photos.json";

/// Where Photos keeps its own files (its key to UUOB, what it last saw).
fn data_dir() -> PathBuf {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    std::env::var_os("XDG_DATA_HOME").map(Into::into).unwrap_or_else(|| home.join(".local/share")).join("app.uuob.photos")
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or_default()
}

fn is_video(p: &Value) -> bool {
    s(p, "type").starts_with("video/")
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[derive(Clone, Debug)]
pub enum Msg {
    Reload,
    /// A place of the sidebar: "all", "videos" or "trash".
    Show(String),
    Settings,
    Open(Option<Value>),
    Step(i32),
    Pick,
    Add(Vec<PathBuf>),
    Remove(Value),
    Restore(Value),
    Confirm(Value),
    Destroy(Value),
    Close,
    /// The window's width changed (phones get three columns).
    Width(i32),
    /// Save the original wherever the user picks (the photo shown when None).
    Save(Option<Value>),
    /// Copy the picture to the clipboard.
    Copy(Option<Value>),
    /// The photo's details.
    Info(Option<Value>),
    /// A short note at the bottom ("Saved", "Copied").
    Note(String),
}

#[derive(Debug)]
pub enum Cmd {
    Loaded(Vec<Value>, Option<Vec<Value>>),
    Adding(usize, usize, Option<String>),
    Done,
    Original(String, Result<Original, String>),
}

#[derive(Debug)]
pub enum Original {
    Image(glib::Bytes),
    Video(PathBuf),
}

pub struct Photos {
    client: Client,
    months: Vec<Value>,
    all: Vec<Value>,
    /// The place shown: "all", "videos" or "trash" (Recently deleted).
    view: String,
    info: AppInfo,
    open: Option<Value>,
    adding: Option<(usize, usize)>,
    error: String,
    made: HashMap<String, String>,
    purged: bool,
    width: i32,
}

pub struct Widgets {
    title: gtk::Label,
    column: gtk::Box,
    add: gtk::Button,
    /// The sidebar's places and the rail's (redrawn with their counts).
    side_rows: gtk::Box,
    rail_rows: gtk::Box,
    close_peek: std::rc::Rc<dyn Fn()>,
    error: gtk::Label,
    groups: gtk::Box,
    progress: gtk::Label,
    pages: gtk::Stack,
    viewer: Viewer,
    toasts: adw::ToastOverlay,
}

/// The full-size view of one photo or video.
pub struct Viewer {
    when: adw::WindowTitle,
    stage: gtk::Stack,
    preview: gtk::Picture,
    picture: gtk::Picture,
    video: gtk::Video,
    delete: gtk::Button,
    restore: gtk::Button,
    destroy: gtk::Button,
    prev: gtk::Button,
    next: gtk::Button,
    save: gtk::Button,
    copy: gtk::Button,
}

fn texture_of(thumb: &str) -> Option<gdk::Texture> {
    let bytes = core::thumb_bytes(thumb.into());
    if bytes.is_empty() {
        return None;
    }
    gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).ok()
}

/// A preview cropped to a square of `size`, as a grid cell shows it (cover).
fn square_of(thumb: &str, size: i32) -> Option<gdk::Texture> {
    let bytes = core::thumb_bytes(thumb.into());
    let pix = gdk_pixbuf::Pixbuf::from_stream(&gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(bytes)), gio::Cancellable::NONE).ok()?;
    let k = size as f64 / pix.width().min(pix.height()).max(1) as f64;
    let (w, h) = (((pix.width() as f64 * k).round() as i32).max(size), ((pix.height() as f64 * k).round() as i32).max(size));
    let scaled = pix.scale_simple(w, h, gdk_pixbuf::InterpType::Bilinear)?;
    let square = scaled.new_subpixbuf((w - size) / 2, (h - size) / 2, size, size);
    Some(gdk::Texture::for_pixbuf(&square))
}

impl Photos {
    /// What the grid shows: the photos, the videos or the deleted ones, by month.
    fn gallery(&self) -> Value {
        core::view_of(&self.all, &self.view)
    }

    fn trash(&self) -> bool {
        self.view == "trash"
    }

    fn shown(&self) -> Vec<Value> {
        let g = self.gallery();
        g["groups"].as_array().into_iter().flatten().flat_map(|g| g["items"].as_array().cloned().unwrap_or_default()).collect()
    }

    /// Phones (under 720 points): three columns, 3 apart, as Photos.svelte's media query.
    /// `width`: the grid's own (beside the sidebar or the rail). Under 560:
    /// three columns that fill it, as on phones.
    fn narrow(&self) -> bool {
        self.width > 0 && self.width < 560
    }

    fn cell_size(&self) -> i32 {
        if self.narrow() { ((self.width - 24 - 6) / 3).max(60) } else { 150 }
    }

    fn render(&self, w: &Widgets, sender: &ComponentSender<Self>) {
        let g = self.gallery();
        if self.narrow() {
            w.column.add_css_class("narrow");
            w.title.add_css_class("compact");
        } else {
            w.column.remove_css_class("narrow");
            w.title.remove_css_class("compact");
        }
        let places = [("all", "Photos", "photo", g["photos"].as_u64().unwrap_or(0)), ("videos", "Videos", "play", g["videos"].as_u64().unwrap_or(0)), ("trash", "Recently deleted", "trash", g["trash"].as_u64().unwrap_or(0))];
        let (name, count) = places.iter().find(|p| p.0 == self.view).map(|p| (p.1, p.3)).unwrap_or(("Photos", 0));
        // The name, its count in grey after it (as Calendar's month and year).
        let grey = if count == 0 { String::new() } else { count.to_string() };
        w.title.set_markup(&format!("<b>{name}</b> <span weight=\"normal\" alpha=\"55%\">{grey}</span>"));
        w.add.set_visible(!self.trash());
        w.add.set_sensitive(self.adding.is_none());
        for b in [&w.side_rows, &w.rail_rows] {
            while let Some(c) = b.first_child() {
                b.remove(&c);
            }
        }
        for (id, label, icon, n) in places {
            // Recently deleted only while something is there.
            if id == "trash" && n == 0 && !self.trash() {
                continue;
            }
            let on = id == self.view;
            let count = gtk::Label::new(Some(&if n == 0 { String::new() } else { n.to_string() }));
            let row = uuob_kit_gtk::sidebar_row(Some(uuob_kit_gtk::icon(icon, 19, 1.75, "text2").upcast_ref()), label, Some(count.upcast_ref()), on);
            let rail = uuob_kit_gtk::rail_icon(icon, label, on);
            for b in [&row, &rail] {
                let (tx, id, close) = (sender.clone(), id.to_string(), w.close_peek.clone());
                b.connect_clicked(move |_| {
                    close();
                    tx.input(Msg::Show(id.clone()));
                });
            }
            w.side_rows.append(&row);
            w.rail_rows.append(&rail);
        }
        w.error.set_label(&self.error);
        w.error.set_visible(!self.error.is_empty());
        w.progress.set_visible(self.adding.is_some());
        if let Some((done, total)) = self.adding {
            w.progress.set_label(&format!("Adding {} of {total}…", (done + 1).min(total)));
        }
        while let Some(c) = w.groups.first_child() {
            w.groups.remove(&c);
        }
        let groups = g["groups"].as_array().cloned().unwrap_or_default();
        if groups.is_empty() {
            w.groups.append(&self.intro(sender));
        }
        for group in groups {
            w.groups.append(&gtk::Label::builder().label(s(&group, "label").to_uppercase()).xalign(0.0).css_classes(["section-label", "month"]).build());
            let (gap, per) = if self.narrow() { (3, 3) } else { (4, 30) };
            let flow = gtk::FlowBox::builder().homogeneous(true).halign(gtk::Align::Start).selection_mode(gtk::SelectionMode::None).column_spacing(gap).row_spacing(gap).max_children_per_line(per).min_children_per_line(if self.narrow() { 3 } else { 2 }).build();
            for p in group["items"].as_array().into_iter().flatten() {
                flow.append(&self.cell(p, sender));
            }
            w.groups.append(&flow);
        }
        w.pages.set_visible_child_name(if self.open.is_some() { "viewer" } else { "grid" });
    }

    /// An empty place: what is (not) there in a few words, Add photos where it helps.
    fn intro(&self, sender: &ComponentSender<Self>) -> gtk::Widget {
        let (icon, headline) = match self.view.as_str() {
            "trash" => ("trash", "Nothing here"),
            "videos" => ("play", "No videos"),
            _ => ("photo", "No photos yet"),
        };
        let add = (self.view == "all").then(|| {
            let add = uuob_kit_gtk::btn("Add photos", Some("plus"), &["primary"]);
            add.set_sensitive(self.adding.is_none());
            let sender = sender.clone();
            add.connect_clicked(move |_| sender.input(Msg::Pick));
            add
        });
        uuob_kit_gtk::empty_state(icon, headline, "orange", add.as_ref()).upcast()
    }

    fn cell(&self, p: &Value, sender: &ComponentSender<Self>) -> gtk::Widget {
        let size = self.cell_size();
        let overlay = gtk::Overlay::builder().width_request(size).height_request(size).halign(gtk::Align::Start).valign(gtk::Align::Start).overflow(gtk::Overflow::Hidden).css_classes(["cell"]).build();
        match square_of(s(p, "thumb"), size) {
            Some(t) => overlay.set_child(Some(&gtk::Picture::builder().paintable(&t).content_fit(gtk::ContentFit::Cover).can_shrink(true).build())),
            None => overlay.set_child(Some(&uuob_kit_gtk::icon(if is_video(p) { "play" } else { "photo" }, 28, 1.75, "text3"))),
        }
        let badge = if self.trash() {
            let d = core::days_left(p["deleted"].as_i64().unwrap_or(0), now());
            Some(format!("{d} {}", if d == 1 { "day" } else { "days" }))
        } else if is_video(p) {
            Some(p["duration"].as_f64().map(core::duration_label).unwrap_or_default())
        } else {
            None
        };
        if let Some(b) = badge {
            let pill = gtk::Box::builder().spacing(4).halign(gtk::Align::End).valign(gtk::Align::End).css_classes(["badge"]).build();
            if !self.trash() {
                pill.append(&uuob_kit_gtk::icon("play", 11, 2.4, "#ffffff"));
            }
            if !b.trim().is_empty() {
                pill.append(&gtk::Label::new(Some(b.trim())));
            }
            overlay.add_overlay(&pill);
        }
        let what = if is_video(p) { "Video" } else { "Photo" };
        let button = gtk::Button::builder().child(&overlay).css_classes(["flat", "cell-button"]).tooltip_text(format!("{what} from {}", core::when_label(p["taken"].as_i64().unwrap_or(0)))).build();
        // Right-click (or a long press): what the viewer offers, without opening it.
        let menu = gtk::Popover::builder().has_arrow(false).css_classes(["web-menu-pop"]).build();
        menu.set_parent(&button);
        let items = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        let choices: Vec<(&str, Msg)> = if self.trash() {
            vec![("Open", Msg::Open(Some(p.clone()))), ("Restore", Msg::Restore(p.clone())), ("Delete forever…", Msg::Confirm(p.clone()))]
        } else {
            let mut c = vec![("Open", Msg::Open(Some(p.clone()))), ("Save…", Msg::Save(Some(p.clone())))];
            if !is_video(p) {
                c.push(("Copy", Msg::Copy(Some(p.clone()))));
            }
            c.push(("Info", Msg::Info(Some(p.clone()))));
            c.push(("Delete", Msg::Remove(p.clone())));
            c
        };
        for (label, msg) in choices {
            let b = gtk::Button::builder().child(&gtk::Label::builder().label(label).xalign(0.0).build()).css_classes(["web-menu-item"]).build();
            let (s, m) = (sender.clone(), menu.clone());
            b.connect_clicked(move |_| {
                m.popdown();
                s.input(msg.clone());
            });
            items.append(&b);
        }
        menu.set_child(Some(&items));
        let right = gtk::GestureClick::builder().button(3).build();
        {
            let menu = menu.clone();
            right.connect_pressed(move |_, _, x, y| {
                menu.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                menu.popup();
            });
        }
        button.add_controller(right);
        let sender = sender.clone();
        let p = p.clone();
        button.connect_clicked(move |_| sender.input(Msg::Open(Some(p.clone()))));
        button.upcast()
    }

    fn show(&self, w: &Widgets, sender: &ComponentSender<Self>) {
        let Some(p) = &self.open else { return };
        let v = &w.viewer;
        v.when.set_title(&core::when_label(p["taken"].as_i64().unwrap_or(0)));
        let deleted = !p["deleted"].is_null();
        v.delete.set_visible(!deleted);
        v.save.set_visible(!deleted);
        // A video can be saved, not copied as a picture.
        v.copy.set_visible(!deleted && !is_video(p));
        v.restore.set_visible(deleted);
        v.destroy.set_visible(deleted);
        let shown = self.shown();
        let i = shown.iter().position(|x| x["id"] == p["id"] && x["container"] == p["container"]).unwrap_or(0);
        v.prev.set_visible(i > 0);
        v.next.set_visible(i + 1 < shown.len());
        // The preview at once; the original replaces it when it has arrived.
        v.preview.set_paintable(texture_of(s(p, "thumb")).as_ref());
        v.video.set_media_stream(None::<&gtk::MediaStream>);
        v.stage.set_visible_child_name("preview");
        let key = format!("{}/{}", s(p, "container"), s(p, "id"));
        let client = self.client.clone();
        let container = s(p, "container").to_string();
        let file = s(p, "file").to_string();
        let video = is_video(p);
        let ext = s(p, "name").rsplit_once('.').map(|(_, e)| e.to_string()).unwrap_or_else(|| "mp4".into());
        sender.oneshot_command(async move {
            let r = client.read_file(Some(&container), &file).await.map_err(|e| e.to_string()).and_then(|bytes| {
                if video {
                    let path = std::env::temp_dir().join(format!("uuob-photos-{}.{ext}", file.replace('/', "_")));
                    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                    Ok(Original::Video(path))
                } else {
                    Ok(Original::Image(glib::Bytes::from_owned(bytes)))
                }
            });
            Cmd::Original(key, r)
        });
    }

    fn step(&self, by: i32) -> Option<Value> {
        let shown = self.shown();
        let p = self.open.as_ref()?;
        let i = shown.iter().position(|x| x["id"] == p["id"] && x["container"] == p["container"])? as i32 + by;
        (i >= 0).then(|| shown.get(i as usize).cloned()).flatten()
    }

    /// The container of a month, created when its first photo is added. Its
    /// id is the same on every device.
    async fn month_container(client: &Client, months: &[Value], made: &mut HashMap<String, String>, key: &str) -> Result<String, String> {
        if let Some(c) = months.iter().find(|c| s(c, "name") == key) {
            return Ok(s(c, "id").into());
        }
        if let Some(id) = made.get(key) {
            return Ok(id.clone());
        }
        let id = client.create_container(key, Some(&core::month_container_key(key.into()))).await.map_err(|e| e.to_string())?;
        let id = id.as_str().unwrap_or_default().to_string();
        made.insert(key.into(), id.clone());
        Ok(id)
    }
}

/// `{ w, h, thumb }` of a picture, upright, as the web version made it.
fn image_preview(path: &std::path::Path) -> Value {
    let Ok(pix) = gdk_pixbuf::Pixbuf::from_file(path) else { return json!({}) };
    let pix = pix.apply_embedded_orientation().unwrap_or(pix);
    let (w, h) = (pix.width() as u32, pix.height() as u32);
    let size = core::thumb_size(w, h);
    let Some(small) = pix.scale_simple(size[0] as i32, size[1] as i32, gdk_pixbuf::InterpType::Bilinear) else { return json!({}) };
    match small.save_to_bufferv("jpeg", &[("quality", "60")]) {
        Ok(jpeg) => json!({ "w": w, "h": h, "thumb": core::jpeg_data_url(jpeg) }),
        Err(_) => json!({ "w": w, "h": h }),
    }
}

/// `{ w, h, duration, thumb }` of a video: its frame at 0.1 s (GStreamer, through GTK).
async fn video_preview(path: PathBuf, widget: gtk::Widget) -> Value {
    let media = gtk::MediaFile::for_filename(&path);
    media.set_muted(true);
    let ready = async {
        while !media.is_prepared() && media.error().is_none() {
            glib::timeout_future(std::time::Duration::from_millis(50)).await;
        }
        if media.error().is_some() {
            return None;
        }
        let duration = media.duration() as f64 / 1_000_000.0;
        media.seek((100_000).min(media.duration().max(0)));
        while media.is_seeking() {
            glib::timeout_future(std::time::Duration::from_millis(50)).await;
        }
        glib::timeout_future(std::time::Duration::from_millis(150)).await;
        let (w, h) = (media.intrinsic_width(), media.intrinsic_height());
        if w <= 0 || h <= 0 {
            return Some(json!({ "duration": duration }));
        }
        let snap = gtk::Snapshot::new();
        media.snapshot(snap.upcast_ref::<gdk::Snapshot>(), w as f64, h as f64);
        let node = snap.to_node()?;
        let renderer = widget.native()?.renderer()?;
        let texture = renderer.render_texture(&node, None);
        let png = texture.save_to_png_bytes();
        let pix = gdk_pixbuf::Pixbuf::from_stream(&gio::MemoryInputStream::from_bytes(&png), gio::Cancellable::NONE).ok()?;
        let size = core::thumb_size(w as u32, h as u32);
        let small = pix.scale_simple(size[0] as i32, size[1] as i32, gdk_pixbuf::InterpType::Bilinear)?;
        let jpeg = small.save_to_bufferv("jpeg", &[("quality", "60")]).ok()?;
        Some(json!({ "w": w, "h": h, "duration": duration, "thumb": core::jpeg_data_url(jpeg) }))
    };
    let timeout = glib::timeout_future_seconds(8);
    futures_select(ready, timeout).await.unwrap_or(json!({}))
}

/// The first of two futures to finish (the second gives up).
async fn futures_select<T>(a: impl std::future::Future<Output = Option<T>>, b: impl std::future::Future<Output = ()>) -> Option<T> {
    use std::pin::pin;
    let mut a = pin!(a);
    let mut b = pin!(b);
    std::future::poll_fn(|cx| {
        if let std::task::Poll::Ready(v) = a.as_mut().poll(cx) {
            return std::task::Poll::Ready(v);
        }
        if b.as_mut().poll(cx).is_ready() {
            return std::task::Poll::Ready(None);
        }
        std::task::Poll::Pending
    })
    .await
}

fn style() {
    let css = gtk::CssProvider::new();
    css.load_from_string(
        "popover.web-menu-pop > contents { background: @uuob_surface2; border: 1px solid alpha(@window_fg_color, 0.14); border-radius: 14px; padding: 6px; box-shadow: 0 12px 32px rgba(0,0,0,0.2); }
         button.web-menu-item { min-height: 36px; padding: 0 12px; border-radius: 9px; background: none; box-shadow: none; font-size: 15px; font-weight: 500; min-width: 160px; }
         button.web-menu-item > label { margin: 0; }
         button.web-menu-item:hover { background: alpha(@window_fg_color, 0.06); }
         .grid-page { padding: 6px 32px 48px; }
         .grid-page.narrow { padding: 0 12px 32px; }
         .grid-page.narrow .large-title { font-size: 30px; }
         .grid-page.narrow .cell, .grid-page.narrow .cell-button { border-radius: 3px; }
         .photos-head { margin-bottom: 18px; }
         .side-col { padding: 4px 12px 12px; }
         .month { margin: 26px 0 10px; }
         .month:first-child { margin-top: 6px; }
         .intro { padding: 44px 24px; margin-top: 24px; }
         .intro .glyph { min-width: 60px; min-height: 60px; border-radius: 18px; background: @uuob_surface2; margin-bottom: 12px; }
         .headline { font-size: 17px; font-weight: 600; }
         .intro-text { margin: 4px 0 18px; }
         button.icon-btn.on-black:hover { background: alpha(white, 0.12); }
         button.icon-btn.nav { min-width: 44px; min-height: 44px; background: rgba(0,0,0,0.4); margin: 14px; }
         .cell { border-radius: 6px; background: @uuob_surface2; }
         flowboxchild { padding: 0; }
         .cell-button { padding: 0; border-radius: 6px; }
         .badge { margin: 6px; padding: 2px 7px; border-radius: 999px; font-size: 12px; font-weight: 600; color: white; background: rgba(0,0,0,0.55); }
         .progress { margin: 20px; padding: 10px 18px; border-radius: 999px; background: @popover_bg_color; box-shadow: 0 2px 8px rgba(0,0,0,0.3); }
         .viewer { background: black; color: white; }
         .viewer-notice { padding: 16px; border-radius: 16px; background: rgba(0,0,0,0.6); color: white; }
         .nav { margin: 12px; }",
    );
    gtk::style_context_add_provider_for_display(&gdk::Display::default().expect("display"), &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

impl relm4::Component for Photos {
    type Init = Client;
    type Input = Msg;
    type Output = ();
    type CommandOutput = Cmd;
    type Root = adw::ApplicationWindow;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder().title("Photos").default_width(1100).default_height(760).width_request(360).height_request(560).build()
    }

    fn init(client: Client, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        uuob_kit_gtk::style();
        style();
        let info = AppInfo {
            id: "photos".into(),
            name: "Photos".into(),
            noun: "photos".into(),
            icon: "image-x-generic-symbolic".into(),
            color: "orange".into(),
            version: core::version(),
        };

        // The sidebar (the kit's): Photos, Videos, Recently deleted; Settings.
        let side_rows = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).build();
        let settings = uuob_kit_gtk::sidebar_row(Some(uuob_kit_gtk::icon("settings", 19, 1.75, "text2").upcast_ref()), "Settings", None, false);
        settings.set_valign(gtk::Align::End);
        settings.set_vexpand(true);
        settings.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Settings)
        });
        let side = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).vexpand(true).css_classes(["side-col"]).build();
        side.append(&side_rows);
        side.append(&settings);
        let side_view = adw::ToolbarView::new();
        let sh = adw::HeaderBar::builder().show_title(false).show_end_title_buttons(false).build();
        sh.pack_start(&uuob_kit_gtk::update_button(FEED, info.version.clone()));
        side_view.add_top_bar(&sh);
        side_view.set_content(Some(&gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&side).build()));
        let rail = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).margin_top(12).vexpand(true).build();
        let rail_rows = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
        rail.append(&rail_rows);
        let rail_settings = uuob_kit_gtk::rail_icon("settings", "Settings (Ctrl+,)", false);
        rail_settings.set_vexpand(true);
        rail_settings.set_valign(gtk::Align::End);
        rail_settings.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Settings)
        });
        rail.append(&rail_settings);

        // The grid: the toolbar (the place and its count, the +), then the months.
        let header = adw::HeaderBar::builder().show_title(false).show_start_title_buttons(false).build();
        let add = uuob_kit_gtk::plus_button("Add photos (Ctrl+N)");
        let (top, title) = uuob_kit_gtk::toolbar("", false, &[], Some(&add));
        add.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Pick)
        });
        let error = gtk::Label::builder().wrap(true).xalign(0.0).css_classes(["error"]).visible(false).build();
        let groups = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let column = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).css_classes(["grid-page"]).build();
        column.append(&top);
        column.append(&error);
        column.append(&groups);
        let clamp = adw::Clamp::builder().maximum_size(1040).tightening_threshold(1040).child(&column).build();
        let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&clamp).build();
        let progress = gtk::Label::builder().halign(gtk::Align::Center).valign(gtk::Align::End).css_classes(["progress"]).visible(false).build();
        let grid_overlay = gtk::Overlay::new();
        grid_overlay.set_child(Some(&scroll));
        grid_overlay.add_overlay(&progress);
        let grid = adw::ToolbarView::new();
        grid.add_top_bar(&header);
        grid.set_content(Some(&grid_overlay));

        // The viewer.
        let when = adw::WindowTitle::new("", "");
        let vheader = adw::HeaderBar::builder().title_widget(&when).show_end_title_buttons(true).build();
        let white = |name: &str, size: i32, tip: &str| gtk::Button::builder().child(&uuob_kit_gtk::icon(name, size, 1.75, "#ffffff")).tooltip_text(tip).css_classes(["icon-btn", "on-black"]).valign(gtk::Align::Center).build();
        let close = white("close", 22, "Close");
        let delete = white("trash", 20, "Delete");
        let restore = uuob_kit_gtk::btn("Restore", None, &[]);
        restore.set_valign(gtk::Align::Center);
        let destroy = uuob_kit_gtk::btn("Delete", None, &["danger"]);
        destroy.set_valign(gtk::Align::Center);
        let save = white("download", 20, "Save… (Ctrl+S)");
        let copy = white("copy", 20, "Copy (Ctrl+C)");
        let info_b = white("info", 20, "Info (I)");
        vheader.pack_start(&close);
        vheader.pack_end(&destroy);
        vheader.pack_end(&restore);
        vheader.pack_end(&delete);
        vheader.pack_end(&save);
        vheader.pack_end(&copy);
        vheader.pack_end(&info_b);
        for (b, m) in [(&save, 0), (&copy, 1), (&info_b, 2)] {
            let s = sender.clone();
            b.connect_clicked(move |_| s.input(match m { 0 => Msg::Save(None), 1 => Msg::Copy(None), _ => Msg::Info(None) }));
        }
        let preview = gtk::Picture::builder().content_fit(gtk::ContentFit::Contain).build();
        let picture = gtk::Picture::builder().content_fit(gtk::ContentFit::Contain).build();
        let video = gtk::Video::builder().autoplay(true).build();
        let missing = gtk::Label::builder()
            .label("The original is on a storage device that can’t be reached right now.")
            .wrap(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["viewer-notice"])
            .build();
        let stage = gtk::Stack::builder().vexpand(true).build();
        stage.add_named(&preview, Some("preview"));
        stage.add_named(&picture, Some("picture"));
        stage.add_named(&video, Some("video"));
        let missing_page = gtk::Overlay::new();
        missing_page.add_overlay(&missing);
        stage.add_named(&missing_page, Some("missing"));
        let prev = white("left", 22, "Previous");
        prev.set_halign(gtk::Align::Start);
        prev.add_css_class("nav");
        let next = white("right", 22, "Next");
        next.set_halign(gtk::Align::End);
        next.add_css_class("nav");
        let vover = gtk::Overlay::new();
        vover.set_child(Some(&stage));
        vover.add_overlay(&prev);
        vover.add_overlay(&next);
        let viewer_view = adw::ToolbarView::builder().css_classes(["viewer"]).build();
        viewer_view.add_top_bar(&vheader);
        viewer_view.set_content(Some(&vover));
        close.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Close)
        });
        prev.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Step(-1))
        });
        next.connect_clicked({
            let s = sender.clone();
            move |_| s.input(Msg::Step(1))
        });
        // Swipe sideways to move between photos (touch screens).
        let swipe = gtk::GestureSwipe::new();
        swipe.set_touch_only(true);
        swipe.connect_swipe({
            let s = sender.clone();
            move |_, vx, _| {
                if vx.abs() > 300.0 {
                    s.input(Msg::Step(if vx < 0.0 { 1 } else { -1 }));
                }
            }
        });
        picture.add_controller(swipe);

        let scaffold = uuob_kit_gtk::SidebarScaffold::new("photos", &side_view, &rail, &grid);
        let pages = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).hhomogeneous(false).vhomogeneous(false).build();
        pages.add_named(&scaffold.widget(), Some("grid"));
        pages.add_named(&viewer_view, Some("viewer"));

        // Keys in the viewer: Escape, ←, →, Ctrl+S, Ctrl+C, Delete, I.
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed({
            let s = sender.clone();
            let pages = pages.clone();
            move |_, key, _, mods| {
                if pages.visible_child_name().as_deref() != Some("viewer") {
                    return glib::Propagation::Proceed;
                }
                let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
                match key {
                    gdk::Key::Escape => s.input(Msg::Close),
                    gdk::Key::Right => s.input(Msg::Step(1)),
                    gdk::Key::Left => s.input(Msg::Step(-1)),
                    gdk::Key::s if ctrl => s.input(Msg::Save(None)),
                    gdk::Key::c if ctrl => s.input(Msg::Copy(None)),
                    gdk::Key::Delete => s.input(Msg::Remove(Value::Null)),
                    gdk::Key::i => s.input(Msg::Info(None)),
                    gdk::Key::space => s.input(Msg::Close),
                    _ => return glib::Propagation::Proceed,
                }
                glib::Propagation::Stop
            }
        });
        root.add_controller(keys);

        // The shared shortcuts (docs/design.md) and Photos' own: the places, Info.
        {
            let m = |msg: fn() -> Msg| -> std::rc::Rc<dyn Fn()> {
                let s = sender.clone();
                std::rc::Rc::new(move || s.input(msg()))
            };
            uuob_kit_gtk::Shortcuts {
                settings: Some(m(|| Msg::Settings)),
                new: Some(("Add photos".into(), m(|| Msg::Pick))),
                find: None,
                toggle_sidebar: Some(scaffold.toggler()),
                refresh: Some(m(|| Msg::Reload)),
                extra: vec![
                    ("Photos".into(), "<Control>1".into(), m(|| Msg::Show("all".into()))),
                    ("Videos".into(), "<Control>2".into(), m(|| Msg::Show("videos".into()))),
                    ("Recently deleted".into(), "<Control>3".into(), m(|| Msg::Show("trash".into()))),
                    ("Info".into(), "<Control>i".into(), m(|| Msg::Info(None))),
                ],
            }
            .install(root.upcast_ref());
        }
        let close_peek = scaffold.closer();
        std::mem::forget(scaffold);
        let gate = Gate::new(info.clone(), client.clone(), {
            let s = sender.clone();
            move || s.input(Msg::Reload)
        });
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&pages));
        gate.set_content(&toasts);
        root.set_content(Some(&gate.widget()));
        std::mem::forget(gate);

        // Live: what changes in UUOB, on any device.
        let mut events = client.subscribe();
        let s = sender.clone();
        glib::spawn_future_local(async move {
            while let Ok(e) = events.recv().await {
                if matches!(e, Event::Changed(_) | Event::State(uuob_client::State::Connected)) {
                    s.input(Msg::Reload);
                }
            }
        });

        let all = client.cached_all("photos");
        let model = Photos { client, months: vec![], all, view: "all".into(), info, open: None, adding: None, error: String::new(), made: HashMap::new(), purged: false, width: 1100 };
        let viewer = Viewer { when, stage, preview, picture, video, delete, restore, destroy, prev, next, save, copy };
        let widgets = Widgets { title, column: column.clone(), add, side_rows, rail_rows, close_peek, error, groups, progress, pages, viewer, toasts };
        model.render(&widgets, &sender);
        // Narrow windows lay the grid out as the web version did on phones.
        root.add_tick_callback({
            let s = sender.clone();
            let last = std::cell::Cell::new(0);
            let area = grid_overlay.clone();
            move |_, _| {
                let w = area.width();
                if w > 0 && w != last.get() {
                    last.set(w);
                    s.input(Msg::Width(w));
                }
                glib::ControlFlow::Continue
            }
        });
        // Buttons whose action needs the photo shown.
        for (button, kind) in [(&widgets.viewer.delete, 0), (&widgets.viewer.restore, 1), (&widgets.viewer.destroy, 2)] {
            let s = sender.clone();
            button.connect_clicked(move |_| {
                s.input(match kind {
                    0 => Msg::Remove(Value::Null),
                    1 => Msg::Restore(Value::Null),
                    _ => Msg::Confirm(Value::Null),
                })
            });
        }
        ComponentParts { model, widgets }
    }

    fn update_with_view(&mut self, w: &mut Widgets, msg: Msg, sender: ComponentSender<Self>, root: &Self::Root) {
        let current = || self.open.clone().unwrap_or(Value::Null);
        match msg {
            Msg::Reload => {
                let client = self.client.clone();
                sender.oneshot_command(async move {
                    let months = client.containers().await.ok().map(|cs| cs.into_iter().filter(|c| c["main"] != json!(true)).collect());
                    let (all, _) = client.all("photos").await;
                    Cmd::Loaded(all, months)
                });
                return;
            }
            Msg::Show(view) => {
                self.view = view;
                self.open = None;
            }
            Msg::Settings => {
                let dialog = uuob_kit_gtk::settings_dialog(&self.info, &self.client, false, vec![], || {});
                dialog.present(Some(root));
                return;
            }
            Msg::Open(p) => {
                self.open = p;
                self.show(w, &sender);
            }
            Msg::Width(px) => {
                let was = (self.narrow(), self.cell_size());
                self.width = px;
                if was == (self.narrow(), self.cell_size()) {
                    return;
                }
            }
            Msg::Close => {
                self.open = None;
                w.viewer.video.set_media_stream(None::<&gtk::MediaStream>);
            }
            Msg::Step(by) => {
                if let Some(p) = self.step(by) {
                    self.open = Some(p);
                    self.show(w, &sender);
                }
            }
            Msg::Pick => {
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Photos and videos"));
                filter.add_mime_type("image/*");
                filter.add_mime_type("video/*");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                let dialog = gtk::FileDialog::builder().title("Add photos").filters(&filters).modal(true).build();
                let s = sender.clone();
                dialog.open_multiple(Some(root), gio::Cancellable::NONE, move |r| {
                    if let Ok(files) = r {
                        let paths: Vec<PathBuf> = (0..files.n_items()).filter_map(|i| files.item(i)?.downcast::<gio::File>().ok()?.path()).collect();
                        if !paths.is_empty() {
                            s.input(Msg::Add(paths));
                        }
                    }
                });
                return;
            }
            Msg::Add(paths) => {
                self.error.clear();
                self.adding = Some((0, paths.len()));
                let client = self.client.clone();
                let months = self.months.clone();
                let mut made = std::mem::take(&mut self.made);
                let tx = sender.clone();
                let widget: gtk::Widget = root.clone().upcast();
                glib::spawn_future_local(async move {
                    let total = paths.len();
                    for (i, path) in paths.iter().enumerate() {
                        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        let r: Result<(), String> = async {
                            let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
                            let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0);
                            let taken = core::taken_or_now(modified, now());
                            let (guess, _) = gio::content_type_guess(Some(path), None::<&[u8]>);
                            let ty = gio::content_type_get_mime_type(&guess).map(|m| m.to_string()).unwrap_or_else(|| "application/octet-stream".into());
                            let container = Photos::month_container(&client, &months, &mut made, &core::month_of(taken)).await?;
                            // A format this device can't draw still gets stored, without a preview.
                            let preview = if ty.starts_with("video/") { video_preview(path.clone(), widget.clone()).await } else { image_preview(path) };
                            let file = client.put_file_path(Some(&container), path, &ty).await.map_err(|e| e.to_string())?;
                            let record = core::new_record(taken, now(), name.clone(), ty, meta.len() as i64, s(&file, "id").into(), preview.to_string());
                            client.put("photos", Some(&container), None, serde_json::from_str(&record).unwrap_or_default()).await.map_err(|e| e.to_string())?;
                            Ok(())
                        }
                        .await;
                        tx.oneshot_command(std::future::ready(Cmd::Adding(i + 1, total, r.err().map(|e| format!("{name} could not be added. {e}")))));
                    }
                    tx.oneshot_command(std::future::ready(Cmd::Done));
                });
            }
            Msg::Remove(arg) => {
                let p = if arg.is_null() { current() } else { arg };
                if p.is_null() {
                    return;
                }
                // Deleting the photo shown moves a photo to Recently Deleted and goes on to the next one, or closes.
                if self.open.as_ref().is_some_and(|o| o["id"] == p["id"]) {
                    self.open = self.step(1);
                    if self.open.is_some() {
                        self.show(w, &sender);
                    }
                }
                let client = self.client.clone();
                sender.oneshot_command(async move {
                    let record = core::mark_deleted(p.to_string(), now());
                    let _ = client.put("photos", Some(s(&p, "container")), None, serde_json::from_str(&record).unwrap_or_default()).await;
                    Cmd::Done
                });
            }
            Msg::Restore(arg) => {
                let p = if arg.is_null() { current() } else { arg };
                self.open = None;
                let client = self.client.clone();
                sender.oneshot_command(async move {
                    let record = core::mark_deleted(p.to_string(), 0);
                    let _ = client.put("photos", Some(s(&p, "container")), None, serde_json::from_str(&record).unwrap_or_default()).await;
                    Cmd::Done
                });
            }
            Msg::Confirm(arg) => {
                let p = if arg.is_null() { current() } else { arg };
                let what = if is_video(&p) { "video" } else { "photo" };
                let s = sender.clone();
                let dialog = uuob_kit_gtk::confirm_sheet(&format!("Delete this {what}?"), "It is removed from all your devices. This can’t be undone.", &[("Cancel", &[], "cancel"), ("Delete", &["danger"], "delete")], false, move |r| {
                    if r == "delete" {
                        s.input(Msg::Destroy(p.clone()));
                    }
                });
                dialog.present(Some(root));
                return;
            }
            Msg::Note(t) => {
                w.toasts.add_toast(adw::Toast::builder().title(&t).timeout(3).build());
                return;
            }
            Msg::Save(p) => {
                let p = p.unwrap_or_else(current);
                let dialog = gtk::FileDialog::builder().title("Save").initial_name(core::save_name(p.to_string())).build();
                let (client, tx) = (self.client.clone(), sender.clone());
                dialog.save(Some(root), None::<&gio::Cancellable>, move |r| {
                    let Ok(file) = r else { return };
                    let Some(path) = file.path() else { return };
                    glib::spawn_future_local(async move {
                        let note = match client.read_file(Some(s(&p, "container")), s(&p, "file")).await {
                            Ok(bytes) => match std::fs::write(&path, bytes) {
                                Ok(()) => "Saved".to_string(),
                                Err(e) => format!("Couldn’t save: {e}"),
                            },
                            Err(_) => "The original is on a storage device that can’t be reached right now.".to_string(),
                        };
                        tx.input(Msg::Note(note));
                    });
                });
                return;
            }
            Msg::Copy(p) => {
                let p = p.unwrap_or_else(current);
                let (client, tx, clip) = (self.client.clone(), sender.clone(), root.clipboard());
                glib::spawn_future_local(async move {
                    let note = match client.read_file(Some(s(&p, "container")), s(&p, "file")).await {
                        Ok(bytes) => match gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)) {
                            Ok(t) => {
                                clip.set_texture(&t);
                                "Copied".to_string()
                            }
                            Err(_) => "This picture can’t be copied.".to_string(),
                        },
                        Err(_) => "The original is on a storage device that can’t be reached right now.".to_string(),
                    };
                    tx.input(Msg::Note(note));
                });
                return;
            }
            Msg::Info(p) => {
                let p = p.unwrap_or_else(current);
                if p.is_null() {
                    return;
                }
                let rows: Vec<Value> = serde_json::from_str(&core::details(p.to_string())).unwrap_or_default();
                // The kit's detail card: Info and ✕, then a row for each detail.
                let (d, body) = uuob_kit_gtk::detail_sheet("Info", &[]);
                let col = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(6).css_classes(["icon-rows"]).build();
                for (i, r) in rows.iter().enumerate() {
                    let value = gtk::Label::builder().label(s(r, "value")).selectable(true).focusable(false).ellipsize(gtk::pango::EllipsizeMode::Middle).xalign(1.0).css_classes(["dim-label"]).build();
                    let row = uuob_kit_gtk::icon_setting_row(Some(s(r, "icon")), s(r, "label"), &value);
                    if i + 1 == rows.len() {
                        row.add_css_class("last");
                    }
                    col.append(&row);
                }
                body.append(&col);
                d.present(Some(root));
                return;
            }
            Msg::Destroy(p) => {
                self.open = None;
                let client = self.client.clone();
                sender.oneshot_command(async move {
                    let c = s(&p, "container").to_string();
                    let _ = client.delete_file(Some(&c), s(&p, "file")).await;
                    let _ = client.delete("photos", Some(&c), s(&p, "id")).await;
                    Cmd::Done
                });
            }
        }
        self.render(w, &sender);
    }

    fn update_cmd_with_view(&mut self, w: &mut Widgets, cmd: Cmd, sender: ComponentSender<Self>, _: &Self::Root) {
        match cmd {
            Cmd::Loaded(all, months) => {
                self.all = all;
                if let Some(m) = months {
                    self.months = m;
                }
                // Anything deleted more than 30 days ago goes for good, once per start.
                if !self.purged && !self.all.is_empty() {
                    self.purged = true;
                    let old: Vec<Value> = serde_json::from_str(&core::expired(Value::Array(self.all.clone()).to_string(), now())).unwrap_or_default();
                    for p in old {
                        sender.input(Msg::Destroy(p));
                    }
                }
            }
            Cmd::Adding(done, total, error) => {
                self.adding = Some((done, total));
                if let Some(e) = error {
                    self.error = e;
                }
            }
            Cmd::Done => {
                self.adding = None;
                sender.input(Msg::Reload);
            }
            Cmd::Original(key, r) => {
                let Some(p) = &self.open else { return };
                if format!("{}/{}", s(p, "container"), s(p, "id")) != key {
                    return;
                }
                let v = &w.viewer;
                match r {
                    Ok(Original::Image(bytes)) => {
                        let loaded = gdk_pixbuf::Pixbuf::from_stream(&gio::MemoryInputStream::from_bytes(&bytes), gio::Cancellable::NONE)
                            .ok()
                            .map(|p| p.apply_embedded_orientation().unwrap_or(p))
                            .map(|p| gdk::Texture::for_pixbuf(&p));
                        match loaded {
                            Some(t) => {
                                v.picture.set_paintable(Some(&t));
                                v.stage.set_visible_child_name("picture");
                            }
                            None => v.stage.set_visible_child_name("missing"),
                        }
                    }
                    Ok(Original::Video(path)) => {
                        v.video.set_filename(Some(&path));
                        v.stage.set_visible_child_name("video");
                    }
                    Err(_) => v.stage.set_visible_child_name("missing"),
                }
                return;
            }
        }
        self.render(w, &sender);
    }
}

fn main() {
    let client = Client::new(App { id: "photos".into(), name: "Photos".into(), android_package: None }, data_dir());
    RelmApp::new("app.uuob.photos").with_args(Vec::new()).run::<Photos>(client);
}
