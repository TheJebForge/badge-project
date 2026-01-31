use crate::character::repr::StateTransitionTrigger;
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{InterState, InterStateImage, InterStateTransition, SharedInterState, SharedInterStateTransition};
use crate::gui::app::editor::{inline_image_resource_picker, inline_validation_error, CharacterEditor};
use crate::gui::app::shared::{MutableStringScope, SharedString};
use crate::gui::app::util::{inline_checkbox, inline_drag_value, inline_duration_value, inline_enum_edit, inline_resource_picker, inline_style_label, inline_text_edit, pick_unique_name, vec_ui, ChangeTracker};
use eframe::emath::{Pos2, Rect};
use eframe::epaint::Shape;
use egui::{vec2, Button, CentralPanel, Color32, ComboBox, Frame, Id, Painter, ScrollArea, SidePanel, Stroke, Style, Ui};
use egui_snarl::ui::{AnyPins, BackgroundPattern, Grid, PinInfo, PinPlacement, PinResponse, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget, WireLayer};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use either::Either;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use strum::EnumIs;
use crate::gui::app::editor::validation::ValidationError;

pub type StateNode = (SharedString, SharedInterState);

pub fn snarl_style() -> SnarlStyle {
    SnarlStyle {
        header_drag_space: Some(vec2(0.0, 0.0)),
        bg_pattern: Some(BackgroundPattern::Grid(Grid::new(vec2(50.0, 50.0), 0.0))),
        bg_pattern_stroke: Some(Stroke::new(1.0, Color32::DARK_GRAY)),
        min_scale: Some(0.5),
        max_scale: Some(1.1),
        collapsible: Some(false),
        pin_placement: Some(PinPlacement::Edge),
        wire_layer: Some(WireLayer::BehindNodes),
        wire_width: Some(8.0),
        wire_smoothness: Some(0.05),
        header_frame: Some(Frame::new()),
        ..SnarlStyle::new()
    }
}

pub fn snarl_from_states(states: &Vec<(SharedString, SharedInterState)>) -> Snarl<StateNode> {
    let mut snarl = Snarl::new();

    // Create nodes
    let mut mapping: HashMap<SharedString, NodeId> = HashMap::new();

    for (name, state) in states {
        let id = snarl.insert_node(
            state.borrow().node_pos.clone(),
            (name.clone(), state.clone()),
        );

        mapping.insert(name.clone(), id);
    }

    // Create connections
    for (this_name, state) in states {
        let borrowed_state = state.borrow();

        let Some(this_node) = mapping.get(this_name) else {
            println!("failed to get lhs mapping for {this_name}");
            continue;
        };

        let out_pin = OutPinId {
            node: this_node.clone(),
            output: 0,
        };

        for transition in &borrowed_state.transitions {
            let borrowed_transition = transition.borrow();

            let Some(other_node) = mapping.get(&borrowed_transition.to_state) else {
                println!(
                    "failed to get rhs mapping for {}",
                    borrowed_transition.to_state
                );
                continue;
            };

            let in_pin = InPinId {
                node: other_node.clone(),
                input: 0,
            };

            snarl.connect(out_pin.clone(), in_pin);
        }
    }

    snarl
}

#[derive(EnumIs, Default)]
pub enum ViewerSelection {
    #[default]
    None,
    SelectedState {
        state: StateNode,
        node: NodeId,
    },
    SelectedTransition {
        parent: StateNode,
        transition: SharedInterStateTransition,
        out_pin_id: OutPinId,
        in_pin_id: InPinId,
    },
}

pub struct StateViewer<'a> {
    selection: &'a mut ViewerSelection,
    states: &'a mut Vec<(SharedString, SharedInterState)>,
    tracker: &'a mut ChangeTracker,
    validation_errors: &'a Vec<ValidationError>,
}

pub const WIRE_COLOR: Color32 = Color32::from_rgb(190, 190, 190);
const SELECTED_COLOR: Color32 = Color32::CYAN;
const SELECTED_BG_COLOR: Color32 = Color32::from_rgb(0, 70, 70);
const ERROR_BG_COLOR: Color32 = Color32::from_rgb(70, 0, 0);

impl StateViewer<'_> {
    fn create_state(&mut self, pos: Pos2, out_pin_id: Option<OutPinId>, snarl: &mut Snarl<StateNode>) {
        let name = pick_unique_name("new".to_string(), &self.states);
        let state = Rc::new(RefCell::new(InterState::default()));
        let node = (name, state);

        self.states.push(node.clone());
        let node_id = snarl.insert_node(pos, node.clone());

        *self.selection = ViewerSelection::SelectedState {
            state: node.clone(),
            node: node_id,
        };

        self.tracker.mark_change();

        if let Some(out) = out_pin_id {
            snarl.connect(out, InPinId { node: node_id, input: 0 });

            let mut borrowed_parent = snarl[out.node].1.borrow_mut();

            borrowed_parent.transitions.push(Rc::new(RefCell::new(
                InterStateTransition {
                    to_state: node.0,
                    trigger: Default::default(),
                }
            )));
        }
    }

    fn does_node_contain_error(&self, node: NodeId, snarl: &Snarl<StateNode>) -> bool {
        for error in self.validation_errors {
            let name = match error {
                ValidationError::DuplicateState(name) => Some(name),
                ValidationError::InvalidAnimationInState(name) => Some(name),
                ValidationError::InvalidNextStateInAnimation(name) => Some(name),
                ValidationError::InvalidImageInState(name) => Some(name),
                ValidationError::InvalidImageInSequenceFrame(name, _) => Some(name),
                _ => None
            };

            if let Some(name) = name {
                if snarl[node].0.str_eq(name) {
                    return true;
                }
            }
            
            if let ValidationError::EmptyStateName = error && snarl[node].0.to_string().is_empty() {
                return true;
            }
        }

        false
    }
}

impl SnarlViewer<StateNode> for StateViewer<'_> {
    fn title(&mut self, node: &StateNode) -> String {
        node.0.to_string()
    }

    fn node_frame(
        &mut self,
        default: Frame,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<StateNode>,
    ) -> Frame {
        let has_error = self.does_node_contain_error(node, snarl);

        if let ViewerSelection::SelectedState {
            node: selected_node,
            ..
        } = &self.selection
        {
            if node == *selected_node {
                return default
                    .stroke(Stroke::new(1.0, SELECTED_COLOR))
                    .fill(if has_error { ERROR_BG_COLOR } else { SELECTED_BG_COLOR });
            }
        }

        if has_error {
            return default
                .stroke(Stroke::new(1.0, Color32::RED))
                .fill(ERROR_BG_COLOR)
        }

        default
    }

    fn show_header(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<StateNode>,
    ) {
        ui.label(self.title(&snarl[node]));
        ui.add_space(4.0);
    }

    fn inputs(&mut self, _node: &StateNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        _ui: &mut Ui,
        _snarl: &mut Snarl<StateNode>,
    ) -> impl SnarlPin + 'static {
        let mut fill_color = WIRE_COLOR;

        if let ViewerSelection::SelectedTransition {
            in_pin_id: selected_in,
            ..
        } = &self.selection
        {
            if *selected_in == pin.id {
                fill_color = SELECTED_COLOR;
            }
        }

        PinInfo::circle()
            .with_fill(fill_color)
            .with_wire_color(WIRE_COLOR)
    }

    fn outputs(&mut self, _node: &StateNode) -> usize {
        1
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        _ui: &mut Ui,
        _snarl: &mut Snarl<StateNode>,
    ) -> impl SnarlPin + 'static {
        let mut fill_color = WIRE_COLOR;

        if let ViewerSelection::SelectedTransition {
            out_pin_id: selected_out,
            ..
        } = &self.selection
        {
            if *selected_out == pin.id {
                fill_color = SELECTED_COLOR;
            }
        }

        PinInfo::circle()
            .with_fill(fill_color)
            .with_wire_color(WIRE_COLOR)
    }

    fn override_wire_color(
        &mut self,
        out_pin: OutPinId,
        in_pin: InPinId,
        _snarl: &Snarl<StateNode>,
    ) -> Option<Color32> {
        if let ViewerSelection::SelectedTransition {
            out_pin_id: selected_out,
            in_pin_id: selected_in,
            ..
        } = &self.selection
        {
            if *selected_out == out_pin && *selected_in == in_pin {
                return Some(SELECTED_COLOR);
            }
        }

        None
    }

    fn final_wire_shapes(
        &mut self,
        in_pins: &HashMap<InPinId, PinResponse>,
        out_pins: &HashMap<OutPinId, PinResponse>,
        shapes: &mut Vec<Shape>,
        snarl: &Snarl<StateNode>
    ) {
        for (id, (_, state)) in snarl.node_ids() {
            let borrowed_state = state.borrow();

            match &borrowed_state.image {
                InterStateImage::Animation { next_state, .. } => {
                    if let Some((next_state_id, _)) = snarl.node_ids().find(|(_, n)| &n.0 == next_state) {
                        let in_pin = &in_pins[&InPinId { node: next_state_id, input: 0 }];
                        let out_pin = &out_pins[&OutPinId { node: id, output: 0 }];

                        shapes.extend(
                            Shape::dashed_line(
                                &[
                                    out_pin.pos,
                                    in_pin.pos
                                ],
                                Stroke::new(3.0, WIRE_COLOR),
                                10.0,
                                10.0
                            )
                        )
                    }
                }
                _ => {}
            }
        }
    }

    fn final_node_rect(&mut self, node: NodeId, _rect: Rect, _ui: &mut Ui, snarl: &mut Snarl<StateNode>) {
        let pos = snarl.get_node_info(node).unwrap().pos;
        snarl[node].1.borrow_mut().node_pos = pos;
    }

    fn node_clicked(&mut self, node: NodeId, snarl: &mut Snarl<StateNode>) {
        *self.selection = ViewerSelection::SelectedState {
            state: snarl[node].clone(),
            node,
        }
    }

    fn has_graph_menu(&mut self, _pos: Pos2, _snarl: &mut Snarl<StateNode>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: Pos2, ui: &mut Ui, snarl: &mut Snarl<StateNode>) {
        if ui.button("New State").clicked() {
            self.create_state(pos, None, snarl);
        }
    }

    fn has_dropped_wire_menu(&mut self, src_pins: AnyPins, _snarl: &mut Snarl<StateNode>) -> bool {
        if let AnyPins::Out(_) = src_pins {
            true
        } else {
            false
        }
    }

    fn show_dropped_wire_menu(&mut self, pos: Pos2, ui: &mut Ui, src_pins: AnyPins, snarl: &mut Snarl<StateNode>) {
        if ui.button("New State").clicked() {
            if let AnyPins::Out(outs) = src_pins {
                self.create_state(pos, Some(outs[0]), snarl);
            }
        }
    }

    fn has_node_menu(&mut self, _node: &StateNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<StateNode>,
    ) {
        let is_last_state = self.states.len() <= 1;

        if ui.add_enabled(!is_last_state, Button::new("Delete")).clicked() {
            if is_last_state {
                return;
            }

            let name = snarl[node].0.clone();

            // Clear all transitions to this state
            for (_, other_state) in &mut *self.states {
                let mut borrowed_other = other_state.borrow_mut();

                if let InterStateImage::Animation { next_state, .. } = &mut borrowed_other.image {
                    if *next_state == name {
                        *next_state = SharedString::from("None")
                    }
                }

                borrowed_other.transitions.retain(|e| {
                    e.borrow().to_state != name
                })
            }

            // Delete the node
            snarl.remove_node(node);
            self.states.retain(|e| e.0 != name);

            *self.selection = ViewerSelection::None;

            self.tracker.mark_change();
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<StateNode>) {
        if from.id.node == to.id.node {
            return;
        }

        if from.remotes.contains(&to.id) {
            return;
        }

        snarl.connect(from.id, to.id);

        let mut parent = snarl[from.id.node].1.borrow_mut();
        let other_name = snarl[to.id.node].0.clone();

        parent.transitions.push(Rc::new(RefCell::new(
            InterStateTransition {
                to_state: other_name,
                trigger: Default::default(),
            }
        )));

        self.tracker.mark_change();
    }

    fn wire_select(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<StateNode>) {
        let parent_node = snarl[from.id.node].clone();
        let Some(transition) = find_transition(&from.id, &to.id, snarl) else {
            return;
        };

        *self.selection = ViewerSelection::SelectedTransition {
            parent: parent_node,
            transition,
            out_pin_id: from.id,
            in_pin_id: to.id,
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<StateNode>) {
        *self.selection = ViewerSelection::None;
        snarl.disconnect(from.id, to.id);

        let mut parent = snarl[from.id.node].1.borrow_mut();
        let other_name = snarl[to.id.node].0.clone();

        parent.transitions
            .retain(|e| e.borrow().to_state != other_name);

        self.tracker.mark_change();
    }

    fn drop_outputs(&mut self, _: &OutPin, _: &mut Snarl<StateNode>) {}

    fn drop_inputs(&mut self, _: &InPin, _: &mut Snarl<StateNode>) {}

    fn draw_background(&mut self, background: Option<&BackgroundPattern>, viewport: &Rect, snarl_style: &SnarlStyle, style: &Style, painter: &Painter, _snarl: &Snarl<StateNode>) {
        if let Some(background) = background {
            background.draw(viewport, snarl_style, style, painter);
        }

        let stroke = Stroke::new(3.0, Color32::WHITE);
        painter.hline(-10.0..=10.0, 0.0, stroke);
        painter.vline(0.0, -10.0..=10.0, stroke);
    }

    fn background_click(&mut self, _rect: Rect, _snarl: &mut Snarl<StateNode>) {
        *self.selection = ViewerSelection::None
    }



}

fn find_transition(
    from: &OutPinId,
    to: &InPinId,
    snarl: &mut Snarl<StateNode>,
) -> Option<SharedInterStateTransition> {
    let parent_node = snarl[from.node].clone();
    let borrowed_parent = parent_node.1.borrow();

    let other_node = snarl[to.node].clone();

    borrowed_parent
        .transitions
        .iter()
        .find(|e| e.borrow().to_state == other_node.0)
        .cloned()
}

impl CharacterEditor {
    pub(crate) fn state_machine_ui(&mut self, ui: &mut Ui) {
        SidePanel::right("node_graph.right")
            .min_width((ui.max_rect().width() * 0.3).max(350.0))
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.heading("Inspector");

                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    const TEXT_WIDTH: f32 = 100.0;

                    match &mut self.graph_selection {
                        ViewerSelection::None => {
                            ui.label("Nothing is selected");
                        }
                        ViewerSelection::SelectedState { state, .. } => {
                            ui.horizontal(|ui| {
                                inline_style_label(ui, "Selected:", TEXT_WIDTH);
                                ui.label(format!("{} (State)", state.0));
                            });

                            state.0.mutate(|str| {
                                inline_text_edit(ui, "Name:", str, TEXT_WIDTH, &mut self.tracker);
                            });

                            inline_validation_error(
                                ui,
                                &self.validation_errors,
                                "Duplicate name!",
                                |err| {
                                    let ValidationError::DuplicateState(name) = err else {
                                        return false;
                                    };

                                    state.0.str_eq(name)
                                },
                                TEXT_WIDTH
                            );

                            inline_validation_error(
                                ui,
                                &self.validation_errors,
                                "Empty name!",
                                |err| {
                                    let ValidationError::EmptyStateName = err else {
                                        return false;
                                    };

                                    state.0.to_string().is_empty()
                                },
                                TEXT_WIDTH
                            );

                            let mut borrowed_state = state.1.borrow_mut();

                            ui.horizontal(|ui| {
                                let id = inline_style_label(ui, "Image Type:", TEXT_WIDTH)
                                    .response
                                    .id;
                                ComboBox::new(id.with("combo"), "")
                                    .selected_text(borrowed_state.image.rich())
                                    .show_ui(ui, |ui| {
                                        let image = &mut borrowed_state.image;

                                        if ui.selectable_label(image.is_none(), "None").clicked() {
                                            *image = InterStateImage::None;
                                            self.tracker.mark_change();
                                        }

                                        if ui
                                            .selectable_label(image.is_single(), "Single")
                                            .clicked()
                                        {
                                            *image = InterStateImage::Single {
                                                image: SharedString::from("None"),
                                                preload: false,
                                            };
                                            self.tracker.mark_change();
                                        }

                                        if ui
                                            .selectable_label(image.is_animation(), "Animation")
                                            .clicked()
                                        {
                                            *image = InterStateImage::Animation {
                                                animation: SharedString::from("None"),
                                                next_state: self.states.first().unwrap().0.clone(),
                                                loop_count: 1,
                                                preload: false,
                                            };
                                            self.tracker.mark_change();
                                        }

                                        if ui
                                            .selectable_label(image.is_sequence(), "Sequence")
                                            .clicked()
                                        {
                                            *image = InterStateImage::Sequence {
                                                frames: vec![],
                                                mode: Default::default(),
                                            };
                                            self.tracker.mark_change();
                                        }
                                    });
                            });

                            ui.separator();

                            match &mut borrowed_state.image {
                                InterStateImage::None => {}
                                InterStateImage::Single {
                                    image,
                                    preload,
                                } => {
                                    inline_image_resource_picker(
                                        ui,
                                        "Image:",
                                        image,
                                        &mut self.images,
                                        &self.location,
                                        TEXT_WIDTH,
                                        &mut self.tracker
                                    );
                                    inline_validation_error(
                                        ui,
                                        &self.validation_errors,
                                        "Invalid image!",
                                        |err| {
                                            let ValidationError::InvalidImageInState(name) = err else {
                                                return false;
                                            };

                                            state.0.str_eq(name)
                                        },
                                        TEXT_WIDTH
                                    );
                                    inline_checkbox(ui, "Preload:", preload, TEXT_WIDTH, &mut self.tracker);
                                }
                                InterStateImage::Animation {
                                    animation,
                                    next_state,
                                    loop_count,
                                    preload,
                                } => {
                                    inline_resource_picker(
                                        ui,
                                        "Animation:",
                                        animation,
                                        &self.animations,
                                        TEXT_WIDTH,
                                        &mut self.tracker
                                    );
                                    inline_validation_error(
                                        ui,
                                        &self.validation_errors,
                                        "Invalid animation!",
                                        |err| {
                                            let ValidationError::InvalidAnimationInState(name) = err else {
                                                return false;
                                            };

                                            state.0.str_eq(name)
                                        },
                                        TEXT_WIDTH
                                    );

                                    inline_resource_picker(
                                        ui,
                                        "Next State:",
                                        next_state,
                                        &self.states,
                                        TEXT_WIDTH,
                                        &mut self.tracker
                                    );
                                    inline_validation_error(
                                        ui,
                                        &self.validation_errors,
                                        "Invalid state!",
                                        |err| {
                                            let ValidationError::InvalidNextStateInAnimation(name) = err else {
                                                return false;
                                            };

                                            state.0.str_eq(name)
                                        },
                                        TEXT_WIDTH
                                    );

                                    inline_drag_value(ui, "Loop Count:", loop_count, TEXT_WIDTH, &mut self.tracker);
                                    inline_checkbox(ui, "Preload:", preload, TEXT_WIDTH, &mut self.tracker);
                                }
                                InterStateImage::Sequence { frames, mode } => {
                                    inline_enum_edit(ui, "Mode:", mode, TEXT_WIDTH, &mut self.tracker);
                                    ui.collapsing("Frames", |ui| {
                                        let images = &mut self.images;

                                        vec_ui(ui, frames, images, |ui, index, frame, images, tracker| {
                                            inline_image_resource_picker(
                                                ui,
                                                "Image:",
                                                &mut frame.image,
                                                images,
                                                &self.location,
                                                TEXT_WIDTH,
                                                tracker
                                            );
                                            inline_validation_error(
                                                ui,
                                                &self.validation_errors,
                                                "Invalid image!",
                                                |err| {
                                                    let ValidationError::InvalidImageInSequenceFrame(name, err_index) = err else {
                                                        return false;
                                                    };

                                                    state.0.str_eq(name) && index == *err_index
                                                },
                                                TEXT_WIDTH
                                            );
                                            inline_duration_value(
                                                ui,
                                                "Duration:",
                                                &mut frame.duration,
                                                TEXT_WIDTH,
                                                tracker
                                            );
                                        }, &mut self.tracker);
                                    });
                                }
                            }
                        }
                        ViewerSelection::SelectedTransition {
                            parent,
                            transition,
                            ..
                        } => {
                            let mut borrowed_transition = transition.borrow_mut();

                            ui.horizontal(|ui| {
                                inline_style_label(ui, "Selected:", TEXT_WIDTH);
                                ui.label(
                                    format!(
                                        "{} -> {} (Transition)",
                                        parent.0,
                                        borrowed_transition.to_state
                                    )
                                );
                            });

                            ui.horizontal(|ui| {
                                let id = inline_style_label(ui, "Trigger:", TEXT_WIDTH)
                                    .response
                                    .id;
                                ComboBox::new(id.with("combo"), "")
                                    .selected_text(borrowed_transition.trigger.rich())
                                    .show_ui(ui, |ui| {
                                        let ty = &mut borrowed_transition.trigger;

                                        if ui.selectable_label(ty.is_clicked(), "Clicked").clicked() {
                                            *ty = StateTransitionTrigger::Clicked;
                                            self.tracker.mark_change();
                                        }

                                        if ui.selectable_label(ty.is_elapsed_time(), "ElapsedTime").clicked() {
                                            *ty = StateTransitionTrigger::ElapsedTime {
                                                duration: 1_000_000
                                            };
                                            self.tracker.mark_change();
                                        }

                                        if ui.selectable_label(ty.is_random(), "Random").clicked() {
                                            *ty = StateTransitionTrigger::Random {
                                                duration_range: Either::Right(1_000_000),
                                                chance: 1,
                                            };
                                            self.tracker.mark_change();
                                        }
                                    });
                            });

                            ui.separator();

                            match &mut borrowed_transition.trigger {
                                StateTransitionTrigger::Clicked => {}
                                StateTransitionTrigger::ElapsedTime { duration } => {
                                    inline_duration_value(ui, "Duration:", duration, TEXT_WIDTH, &mut self.tracker);
                                }
                                StateTransitionTrigger::Random {
                                    duration_range,
                                    chance
                                } => {
                                    ui.horizontal(|ui| {
                                        inline_style_label(ui, "Duration Type:", TEXT_WIDTH);

                                        if ui.radio(duration_range.is_left(), "Range").clicked() {
                                            *duration_range = Either::Left((0_500_000, 1_000_000));
                                            self.tracker.mark_change();
                                        }

                                        if ui.radio(duration_range.is_right(), "Single").clicked() {
                                            *duration_range = Either::Right(1_000_000);
                                            self.tracker.mark_change();
                                        }
                                    });

                                    match duration_range {
                                        Either::Left((from, to)) => {
                                            inline_duration_value(ui, "From:", from, TEXT_WIDTH, &mut self.tracker);
                                            inline_duration_value(ui, "To:", to, TEXT_WIDTH, &mut self.tracker);
                                        }
                                        Either::Right(duration) => {
                                            inline_duration_value(ui, "Duration:", duration, TEXT_WIDTH, &mut self.tracker);
                                        }
                                    }

                                    inline_drag_value(ui, "Chance (1 in X):", chance, TEXT_WIDTH, &mut self.tracker);
                                }
                            }
                        }
                    }
                });
            });

        CentralPanel::default().show(ui.ctx(), |ui| {
            SnarlWidget::new()
                .id(Id::new("state_machine.graph"))
                .style(self.graph_style)
                .show(&mut self.state_graph, &mut StateViewer {
                    selection: &mut self.graph_selection,
                    states: &mut self.states,
                    tracker: &mut self.tracker,
                    validation_errors: &self.validation_errors
                }, ui);
        });
    }
}
