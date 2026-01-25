mod start;
mod editor;
pub mod util;
mod shared;

use crate::gui::app::start::StartScreen;
use eframe::{App, CreationContext, Frame};
use egui::{CentralPanel, Color32, Context, ScrollArea, TopBottomPanel, Ui, Window};

pub struct GuiApp {
    show_style_ui: bool,
    tab: BoxedGuiPage
}

impl GuiApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        cc.egui_ctx.style_mut(|style| {
            let w = &mut style.visuals.widgets;
            let text = Color32::from_gray(220);

            w.noninteractive.fg_stroke.color = text;
            w.inactive.fg_stroke.color = text;
            style.visuals.slider_trailing_fill = true;
            style.interaction.selectable_labels = false;
        });

        egui_extras::install_image_loaders(&cc.egui_ctx);

        Self {
            show_style_ui: false,
            tab: StartScreen::new()
        }
    }
}

#[derive(Default)]
pub enum PageResponse {
    #[default]
    Nothing,
    SwitchPage(Box<dyn GuiPage>)
}

pub trait GuiPage {
    fn show(&mut self, ui: &mut Ui) -> PageResponse;
}

pub type BoxedGuiPage = Box<dyn GuiPage>;

impl App for GuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.show_style_ui {
            Window::new("Style UI")
                .show(ctx, |ui| {
                    ScrollArea::vertical()
                        .show(ui, |ui| {
                            ctx.style_ui(ui, ctx.theme());
                            ctx.inspection_ui(ui);
                        })
                });
        }

        TopBottomPanel::bottom("bottom")
            .show(ctx, |ui| {
                ui.toggle_value(&mut self.show_style_ui, "Style UI");
            });

        CentralPanel::default()
            .show(ctx, |ui| {
                match self.tab.as_mut().show(ui) {
                    PageResponse::SwitchPage(page) => {
                        self.tab = page;
                    }

                    _ => {}
                }
            });
    }
}