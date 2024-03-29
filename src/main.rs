use eframe::egui;
use egui::color_picker::Alpha;
use egui_dock::{DockArea, DockState, NodeIndex, Style};

use std::path::PathBuf;
use egui_file_dialog::FileDialog;

struct TabViewer<'a> {
    state: &'a mut SharedState
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab.as_str() {
            "Assets" => self.file_tab(ui),
            "Hierarchy" => self.hierarchy_tab(ui),
            "Timeline" => self.timeline_tab(ui),
            "Timeline Options" => self.timeline_options_tab(ui),
            "Scene" => self.scene_tab(ui),
            _ => {
                ui.label(format!("Empty {tab} contents"));
            }
        }
    }
}

impl TabViewer<'_> {
    fn table_ui(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{TableBuilder, Column};

        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(30.0).at_least(30.0).clip(true))
            .column(Column::initial(150.0).clip(true))
            .column(Column::auto().clip(true))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.centered_and_justified(|ui| {
                        ui.strong("Used");
                    });
                });
                header.col(|ui| {
                    ui.strong("File");
                });
                header.col(|ui| {
                    ui.strong("Size (B)");
                });
                header.col(|ui| {
                    ui.strong("Thumbnail");
                });
            })
            .body(|body| {
                body.rows(
                    text_height,
                    self.state.file.files.len(), |mut row| {
                        let row_index = row.index();

                        row.set_selected(
                            (self.state.file.selected_file.is_some() && self.state.file.files[row_index].path == *self.state.file.selected_file.as_ref().unwrap())
                            || self.state.file.files[row_index].selected
                        );

                        row.col(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.checkbox(&mut self.state.file.files[row_index].used, "");
                            });
                        });

                        let path = &self.state.file.files[row_index].path;
                        row.col(|ui| {
                            ui.label(path.file_name().unwrap().to_str().unwrap());
                            ui.small(path.to_str().unwrap());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", path.metadata().unwrap().len()));
                        });
                        row.col(|ui| {
                            // TODO: implement thumbnails for some media types
                            ui.label("todo");
                        });

                        if row.response().clicked() {
                            self.state.file.files[row_index].selected = !self.state.file.files[row_index].selected;
                        }
                    })
            });
    }

    fn file_tab(&mut self, ui: &mut egui::Ui) {
        if ui.add_sized([ui.min_rect().width(), 10.], egui::Button::new("Add file")).clicked() {
            self.state.file.file_dialog.select_file();
        }

        ui.add_space(15.0);
        self.table_ui(ui);

        if let Some(path) = self.state.file.file_dialog.update(ui.ctx()).selected() {
            self.state.file.selected_file = Some(path.to_path_buf());

            if let Some(l) = self.state.file.files.last() {
                if path == l.path || self.state.file.files.iter().any(|f| f.path == path.to_path_buf()) {
                    return;
                }
            }

            self.state.file.files.push(File::new(path.to_path_buf()));
            // TODO: fix id conflict when assets have the same name
            self.state.hierarchy.assets.push(Asset::Object(ObjectConfig::new(path.file_stem().unwrap().to_str().unwrap())))
        }
    }

    fn hierarchy_tab(&mut self, ui: &mut egui::Ui) {
        // TODO: use context menus instead
        if ui.button("Add asset").clicked() || self.state.hierarchy.adding {
            self.state.hierarchy.adding = true;

            ui.label("new asset name");
            let response = ui.add(egui::TextEdit::singleline(&mut self.state.hierarchy.new_name));
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                // TODO: prevent name/id conflicts
                self.state.hierarchy.assets.push(Asset::Object(ObjectConfig::new(self.state.hierarchy.new_name.as_str())));

                self.state.hierarchy.new_name.clear();
                self.state.hierarchy.adding = false;
            }
        }

        ui.add_space(1.0);

        for asset in self.state.hierarchy.assets.iter_mut() {
            match asset {
                Asset::Object(obj) => obj.obj_ui(ui),
                _ => {}
            }
        }
    }

    // TODO: viewport and possibly make this a custom widget?
    fn timeline_tab(&mut self, ui: &mut egui::Ui) {
        for asset in self.state.hierarchy.assets.iter_mut() {
            match asset {
                Asset::Object(obj) => {
                    let block = ui.add(TimelineState::block(obj));

                    obj.appointment += block.drag_delta().x;
                    obj.appointment = obj.appointment.max(0.0);
                    if block.clicked() {
                        obj.selected = !obj.selected;
                    }
                },
                _ => {}
            }
        }
    }

    fn timeline_options_tab(&mut self, ui: &mut egui::Ui) {
        if ui.button(if self.state.timeline.playing {"Pause"} else {"Play"}).clicked() {
            self.state.timeline.playing = !self.state.timeline.playing;
        }

        ui.strong(format!("time: {}", self.state.timeline.time));
    }

    fn scene_tab(&mut self, ui: &mut egui::Ui) {

    }
}

#[derive(Default, Debug)]
enum Track {
    #[default]
    Video,
    Audio
}

#[derive(Default)]
struct TimelineState {
    playing: bool,
    time: f32,

    tracks: Vec<Track>,
}

impl TimelineState {
    // TODO: fix weird stuff happening with mutliple blocks
    fn display_block(ui: &mut egui::Ui, obj: &ObjectConfig) -> egui::Response {
        let size = obj.to_rect(ui);
        let mut response = ui.allocate_rect(size, egui::Sense::click_and_drag());

        if response.clicked() {
            // TODO: select for scene view and stuff
            response.mark_changed();
        }
        if response.dragged() {
            // TODO: change appointment, duration (if dragged on the edge), etc.
            response.mark_changed();
        }

        if ui.is_rect_visible(size) {
            // TODO: add label with obj name in the rectangle
            ui.painter().rect_filled(
                size,
                10.0,
                obj.colour
            );
        }

        if obj.selected {
            ui.painter().rect_stroke(
                size,
                10.0,
                egui::Stroke::new(1.0, egui::Color32::WHITE)
            );
        }

        response
    }

    fn block<'a> (obj: &'a ObjectConfig) -> impl egui::Widget + 'a {
        move |ui: &mut egui::Ui| Self::display_block(ui, obj)
    }
}

#[derive(Default)]
struct HierarchyState {
    adding: bool,       // is adding a new asset
    new_name: String,   // new asset name

    assets: Vec<Asset>
}

enum Asset {
    Object(ObjectConfig),
    Media((ObjectConfig, PathBuf))  // e.g. image, video
}

impl Default for Asset {
    fn default() -> Self {
        Self::Object(Default::default())
    }
}

#[derive(Default)]
struct ObjectConfig {
    name: String,

    appointment: f32,
    duration: f32,
    track: (usize, Track),

    position: egui::Vec2,
    size: u16,
    colour: egui::Color32,

    selected: bool
}

macro_rules! add_inf_drag {
    ($ui: ident, $num: expr, $start: expr, $prefix: expr) => {
        $ui.add(
            egui::DragValue::new(&mut $num)
            .speed(0.1)
            .clamp_range($start..=f64::INFINITY)
            .prefix($prefix)
        );
    };
}

impl ObjectConfig {
    fn new(name: &str) -> Self {
        let mut obj: Self = Default::default();
        obj.name = name.to_string();

        obj
    }

    fn obj_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.name.to_owned())
            .show(ui, |ui| {
                const GAP: f32 =  15.0;

                ui.strong("Timeline");
                add_inf_drag!(ui, self.appointment, 0.0, "Appointment: ");
                add_inf_drag!(ui, self.duration, 0.0, "Duration: ");

                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.label("Track (");
                    add_inf_drag!(ui, self.track.0, 0.0, "");
                    ui.label(format!(", {:?})", self.track.1));
                });
                ui.add_space(GAP);

                ui.strong("Position");
                add_inf_drag!(ui, self.position.x, f64::NEG_INFINITY, "x: ");
                add_inf_drag!(ui, self.position.y, f64::NEG_INFINITY, "y: ");
                ui.add_space(GAP);

                ui.strong("Size");
                ui.add(egui::Slider::new(&mut self.size, 0..=100).text("Size"));
                ui.add_space(GAP);

                ui.strong("Colour");
                egui::color_picker::color_picker_color32(ui, &mut self.colour, Alpha::Opaque);
            });
    }

    fn to_rect(&self, ui: &egui::Ui) -> egui::Rect {
        let padding = 20;
        let offset = self.appointment + ui.available_size().x/* ui.available_rect_before_wrap().min.x */;
        let track_width = 50;
        let track = ui.available_size().y + (track_width * self.track.0 + padding) as f32;
        egui::Rect {
            min: egui::Pos2::new(offset, track),
            max: egui::Pos2::new(offset + self.duration, track + track_width as f32)
        }
    }
}

// keeps track of file state
#[derive(Default)]
struct FileState {
    file_dialog: FileDialog,
    selected_file: Option<PathBuf>,

    files: Vec<File>
}

#[derive(Default)]
struct File {
    path: PathBuf,
    used: bool,
    selected: bool
}

impl File {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            used: true,
            selected: false
        }
    }
}

// shared state between SullaState (app) and TabViewer (tabs)
#[derive(Default)]
struct SharedState {
    file: FileState,
    hierarchy: HierarchyState,
    timeline: TimelineState
}

struct SullaState {
    tree: DockState<String>,
    state: SharedState
}

impl Default for SullaState {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["Timeline".to_owned()]);

        let [timeline, _assets] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.25, vec!["Hierarchy".to_owned(), "Assets".to_owned()]);
        let [_, _timeline_options] =
            tree.main_surface_mut()
                .split_left(timeline, 0.20, vec!["Timeline Options".to_owned()]);
        let [_, _player] =
            tree.main_surface_mut()
                .split_above(timeline, 0.5, vec!["Player".to_owned(), "Scene".to_owned()]);

        Self {
            tree,
            state: Default::default()
        }
    }
}

impl eframe::App for SullaState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut tv = TabViewer {
            state: &mut self.state
        };

        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tv);
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "sulla",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::<SullaState>::default())
    )
}
