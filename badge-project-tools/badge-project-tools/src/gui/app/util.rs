use crate::character::util::AsRichText;
use crate::gui::app::shared::MutableStringScope;
use eframe::emath::{Align, Numeric, vec2};
use eframe::epaint::Color32;
use egui::{ComboBox, DragValue, Frame, InnerResponse, Label, Layout, Response, Ui, WidgetText};
use image::{ColorType, DynamicImage, ImageReader};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::{env, io};
use strum::IntoEnumIterator;

pub const SPACING: f32 = 6.0;

pub fn list_ui<K, T>(
    ui: &mut Ui,
    map: &mut Vec<(K, T)>,
    element_fn: impl Fn(&mut Ui, usize, &mut K, &mut T),
) where
    K: From<String> + Display + MutableStringScope,
    T: Default,
{
    let pick_name = || -> K {
        let mut name = "new".to_string();

        let mut count = 1;
        while map.iter().any(|(e, _)| e.refer(|e| e == &name)) {
            name = format!("{name}{count}");
            count += 1;
        }

        name.into()
    };

    if ui.button("+").clicked() {
        map.insert(0, (pick_name(), T::default()));
    }

    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
        Frame::new()
            .fill(ui.style().visuals.text_edit_bg_color())
            .corner_radius(SPACING)
            .inner_margin(SPACING)
            .show(ui, |ui| {
                if map.is_empty() {
                    ui.vertical_centered_justified(|ui| {
                        ui.label("Empty".rich().size(16.0).color(Color32::GRAY));
                    });

                    return;
                }

                let mut to_delete: Option<usize> = None;

                for (index, (key, value)) in map.iter_mut().enumerate() {
                    Frame::new()
                        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
                        .corner_radius(SPACING / 2.0)
                        .inner_margin(SPACING)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("X").clicked() {
                                    to_delete = Some(index)
                                }

                                ui.add_space(SPACING);

                                key.mutate(|key| {
                                    ui.text_edit_singleline(key);
                                });

                                ui.take_available_width();
                            });

                            ui.separator();

                            element_fn(ui, index, key, value);
                        });
                }

                if let Some(index) = to_delete {
                    map.remove(index);
                }
            });
    });
}

pub fn inline_style_label(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    width: f32,
) -> InnerResponse<Response> {
    ui.allocate_ui_with_layout(
        vec2(width, 15.0),
        Layout::top_down_justified(Align::Max),
        |ui| ui.label(label),
    )
}

pub fn inline_drag_value(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    numeric: &mut impl Numeric,
    width: f32,
) -> InnerResponse<()> {
    ui.horizontal_top(|ui| {
        inline_style_label(ui, label, width);
        ui.add(DragValue::new(numeric));
    })
}

pub fn inline_checkbox(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut bool,
    width: f32,
) -> InnerResponse<()> {
    ui.horizontal_top(|ui| {
        inline_style_label(ui, label, width);
        ui.checkbox(value, "");
    })
}

pub fn inline_color_edit_rgb_tuple(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    color: &mut (u8, u8, u8),
    width: f32,
) -> InnerResponse<()> {
    let mut arr: [u8; 3] = (*color).into();

    let resp = ui.horizontal_top(|ui| {
        inline_style_label(ui, label, width);
        ui.color_edit_button_srgb(&mut arr);
    });

    color.0 = arr[0];
    color.1 = arr[1];
    color.2 = arr[2];

    resp
}

pub fn inline_enum_edit<T>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut T,
    width: f32,
) -> InnerResponse<()>
where
    T: IntoEnumIterator + Display + PartialEq,
{
    ui.horizontal(|ui| {
        let id = inline_style_label(ui, label, width).response.id;
        ComboBox::new(id.with("combo"), "")
            .selected_text(value.to_string())
            .show_ui(ui, |ui| {
                for variant in T::iter() {
                    let label = variant.to_string();
                    ui.selectable_value(value, variant, label);
                }
            });
    })
}

pub fn inline_text_edit(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut String,
    width: f32,
) -> InnerResponse<()> {
    ui.horizontal(|ui| {
        inline_style_label(ui, label, width);
        ui.text_edit_singleline(value);
    })
}

pub fn disabled_text_edit(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    leave_space: f32,
) -> InnerResponse<InnerResponse<()>> {
    Frame::new()
        .fill(ui.style().visuals.text_edit_bg_color())
        .stroke(ui.style().visuals.widgets.inactive.bg_stroke)
        .corner_radius(ui.style().visuals.widgets.inactive.corner_radius)
        .inner_margin(ui.style().spacing.button_padding)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let possible_width = ui
                    .max_rect()
                    .width()
                    .min(ui.style().spacing.text_edit_width - leave_space);

                let label = ui.allocate_ui(vec2(possible_width, 15.0), |ui| {
                    ui.add(Label::new(label).halign(Align::Max).truncate())
                });
                let remaining = possible_width - label.response.rect.width() - 8.0;
                let to_add = ui.max_rect().width().min(remaining);

                if to_add > 0.0 {
                    ui.add_space(to_add)
                }
            })
        })
}

pub(crate) const BUTTON_WIDTH: f32 = 80.0;

pub fn inline_folder_picker(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut PathBuf,
    location: impl AsRef<Path>,
    width: f32,
) -> InnerResponse<()> {
    let location = location.as_ref().to_path_buf();

    ui.horizontal(|ui| {
        inline_style_label(ui, label, width);
        if ui.button("Pick Folder").clicked() {
            if let Some(picked_folder) = rfd::FileDialog::new()
                .set_title("Pick folder that contains frame images")
                .set_directory(&location)
                .pick_folder()
            {
                match picked_folder.strip_prefix(&location) {
                    Ok(relative) => {
                        *value = relative.to_path_buf();
                    }
                    Err(_) => *value = picked_folder,
                }
            }
        }
        disabled_text_edit(ui, value.to_string_lossy(), BUTTON_WIDTH);
    })
}

pub fn load_image(path: impl AsRef<Path>) -> anyhow::Result<DynamicImage> {
    Ok(ImageReader::open(path)?.decode()?)
}

pub fn load_image_or_black(path: impl AsRef<Path>) -> DynamicImage {
    load_image(path.as_ref()).unwrap_or_else(|_| DynamicImage::new(320, 320, ColorType::Rgb8))
}

pub fn inline_resource_picker<K, V>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut K,
    collection: &Vec<(K, V)>,
    width: f32,
) -> InnerResponse<()>
where
    K: Display + PartialEq + Clone,
{
    ui.horizontal(|ui| {
        let id = inline_style_label(ui, label, width).response.id;
        ComboBox::new(id.with("combo"), "")
            .selected_text(value.to_string())
            .show_ui(ui, |ui| {
                for (k, _) in collection {
                    if ui.selectable_label(value == k, k.rich()).clicked() {
                        *value = k.clone();
                    }
                }
            });
    })
}
