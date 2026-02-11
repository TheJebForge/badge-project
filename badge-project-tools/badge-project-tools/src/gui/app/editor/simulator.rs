use crate::character::repr::{Animation, SequenceMode};
use crate::character::util::AsRichText;
use crate::gui::app::editor::intermediate::{InterAction, InterActionType, InterSequence, InterStateImage, SharedInterState, SharedLoadedImage};
use crate::gui::app::editor::nodes::{StateNode, WIRE_COLOR};
use crate::gui::app::editor::validation::ValidationError;
use crate::gui::app::shared::SharedString;
use crate::gui::app::util::SPACING;
use eframe::epaint::{Shape, Stroke};
use egui::{vec2, Align2, CentralPanel, Color32, FontId, Frame, Id, Painter, Rect, ScrollArea, Sense, SidePanel, Style, TopBottomPanel, Ui};
use egui_snarl::ui::{
    BackgroundPattern, PinInfo, PinResponse, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget,
};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use num_format::{Locale, ToFormattedString};
use std::cmp::{Ordering, PartialEq};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Deref;
use std::rc::{Rc, Weak};
use strum::EnumIs;

pub fn simulator_ui(
    ui: &mut Ui,
    simulator_state: &mut Option<SimulatorState>,
    images: &Vec<(SharedString, SharedLoadedImage)>,
    sequences: &Vec<(SharedString, InterSequence)>,
    animations: &Vec<(SharedString, Animation)>,
    states: &Vec<(SharedString, SharedInterState)>,
    state_graph: &mut Snarl<(SharedString, SharedInterState)>,
    graph_style: SnarlStyle,
    actions: &Vec<(String, InterAction)>,
    default_state: &SharedString,
    validations: &Vec<ValidationError>,
) {
    if let Some(state) = simulator_state  {
        let exit_requested = Simulator {
            sim_state: state,
            images,
            sequences,
            animations,
            actions,
            states,
            snarl: state_graph,
            graph_style,
        }.show_ui(ui);

        if exit_requested {
            *simulator_state = None
        }
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 20.0);
            ui.heading("Simulator session is closed");

            ui.add_enabled_ui(
                !validations.contains(&ValidationError::InvalidDefaultState),
                |ui| {
                    if ui.button("Start Simulator").clicked() {
                        let Some((_, default_state_info)) =
                            states.iter().find(|(k, _)| k == default_state) else {
                            eprintln!("Default state is not found??");
                            return;
                        };

                        let borrowed_state = default_state_info.borrow();

                        *simulator_state = Some(SimulatorState {
                            status: Default::default(),
                            current_layer: borrowed_state.layer,
                            current_state: default_state.clone(),
                            next_state: None,
                            possible_transitions: vec![],
                            possible_actions: vec![],
                            current_image: None,
                            new_layer_images: Default::default(),
                            new_layer_animations: Default::default(),
                            loaded_layer_images: Default::default(),
                            loaded_layer_animations: Default::default(),
                            layer_images_to_remove: Default::default(),
                            layer_animations_to_remove: Default::default(),
                            loaded_images: vec![],
                            prepared_images: vec![],
                            allocator: AllocatorState::default(),
                        })
                    }
                },
            );
        });
    }
}

pub struct Simulator<'a> {
    pub sim_state: &'a mut SimulatorState,
    pub images: &'a Vec<(SharedString, SharedLoadedImage)>,
    pub sequences: &'a Vec<(SharedString, InterSequence)>,
    pub animations: &'a Vec<(SharedString, Animation)>,
    pub actions: &'a Vec<(String, InterAction)>,
    pub states: &'a Vec<(SharedString, SharedInterState)>,
    pub snarl: &'a mut Snarl<(SharedString, SharedInterState)>,
    pub graph_style: SnarlStyle,
}

impl Simulator<'_> {
    pub fn show_ui(&mut self, ui: &mut Ui) -> bool {
        let mut exit_requested = false;

        if self.sim_state.status.is_uninitialized() {
            if let None = self.preload() {
                self.sim_state.error("Out of memory during preload!");
                return exit_requested;
            }

            let current = self.sim_state.current_state.clone();
            if !self.schedule_or_switch(&current) {
                self.sim_state
                    .error("Out of memory trying to load default state!");
                return exit_requested;
            }

            self.switch_to_scheduled();

            self.sim_state.status = SimulatorStatus::Loaded;
        }

        TopBottomPanel::top("simulator.header")
            .resizable(false)
            .show(ui.ctx(), |ui| {
                if ui.button("Close Session").clicked() {
                    exit_requested = true;
                }

                visualize_allocator(ui, &self.sim_state);
            });

        match &self.sim_state.status {
            SimulatorStatus::Error(err) => {
                CentralPanel::default()
                    .show(ui.ctx(), |ui| {
                        ui.heading(format!("Error: {err}").rich().color(Color32::RED));
                    });
            }

            SimulatorStatus::Loaded => {
                SidePanel::left("simulator.traversal")
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label(format!("Current State: {}", self.sim_state.current_state));
                        ui.label(format!("Current Layer: {}", self.sim_state.current_layer));

                        if let Some(next) = &self.sim_state.next_state {
                            ui.add_space(SPACING);

                            ui.label(format!("Next State: {}", next));

                            if ui.button("Switch to next state").clicked() {
                                self.switch_to_scheduled();
                            }
                        }

                        ui.add_space(SPACING);

                        ui.add_enabled_ui(self.sim_state.next_state.is_none(), |ui| {
                            if let Some(next_state) = Self::next_state_ui(
                                ui,
                                "Possible Transitions:",
                                &self.sim_state.possible_transitions,
                            ) {
                                if !self.schedule_or_switch(&next_state) {
                                    self.sim_state.error("Out of memory trying to cook state!")
                                }
                            }

                            if let Some(next_state) = Self::next_state_ui(
                                ui,
                                "Possible Action Switches:",
                                &self.sim_state.possible_actions,
                            ) {
                                if !self.schedule_or_switch(&next_state) {
                                    self.sim_state.error("Out of memory trying to cook state!")
                                }
                            }
                        });
                    });

                CentralPanel::default()
                    .show(ui.ctx(), |ui| {
                        SnarlWidget::new()
                            .id(Id::new("simulator.graph"))
                            .style(self.graph_style)
                            .show(&mut self.snarl, &mut SimulatorNodeViewer {
                                current_state: &self.sim_state.current_state,
                                next_state: self.sim_state.next_state.as_ref(),
                            }, ui);
                    });
            }
            _ => {}
        }

        exit_requested
    }

    fn next_state_ui<'a>(
        ui: &mut Ui,
        label: impl Display,
        transitions: &Vec<StateSwitchInfo>,
    ) -> Option<SharedString> {
        let label = label.to_string();

        ui.label(&label);
        ui.allocate_ui(
            vec2(ui.max_rect().width() / 2.0, ui.max_rect().height() / 3.0),
            |ui| {
                Frame::canvas(ui.style())
                    .show(ui, |ui| {
                        ui.allocate_exact_size(vec2(200.0, 0.0), Sense::empty());
                        ScrollArea::vertical()
                            .id_salt(&label)
                            .show(ui, |ui| {
                                for info in transitions {
                                    let text = format!(
                                        "{}{}{}",
                                        info.name,
                                        if info.is_dynamic { " (cook)" } else { "" },
                                        if info.is_layer_switch {" (switch layer)"} else {""}
                                    );

                                    if ui.button(text).clicked() {
                                        return Some(info.name.clone());
                                    }
                                }

                                None
                            })
                            .inner
                    })
                    .inner
            },
        )
        .inner
    }

    pub fn is_dynamic_or_layer_switch(&self, state: &SharedString) -> Option<(bool, bool)> {
        let Some((_, state)) = self.states.iter().find(|(k, _)| k == state) else {
            return None;
        };

        let state = state.borrow();

        let is_layer_switch = state.layer != self.sim_state.current_layer;

        let is_dynamic = match &state.image {
            InterStateImage::None => false,
            InterStateImage::Single { load_mask, .. } =>
                load_mask & state.layer == 0,
            InterStateImage::Animation {
                load_mask, animation, ..
            } => {
                if load_mask & state.layer != 0 {
                    false
                } else {
                    let Some((_, animation)) = self.animations.iter().find(|(k, _)| k == animation)
                    else {
                        return None;
                    };

                    animation.mode.is_from_ram()
                }
            }
            InterStateImage::Sequence { load_mask, .. } =>
                load_mask & state.layer == 0,
        };

        Some((is_dynamic, is_layer_switch))
    }

    pub fn cook_state(&mut self, state: &SharedString) -> bool {
        let Some((name, state)) = self.states.iter().find(|(k, _)| k == state) else {
            return true;
        };

        let state = state.borrow();

        self.sim_state.next_state = Some(name.clone());
        self.sim_state.prepared_images.clear();

        match &state.image {
            InterStateImage::None => {}
            InterStateImage::Single { image, load_mask } => {
                if load_mask & self.sim_state.current_layer != 0 {
                    return true;
                }

                let Some((_, image)) = self.images.iter().find(|(k, _)| k == image) else {
                    return true;
                };

                let image = image.borrow();

                let size = calc_required_space(image.width, image.height, image.upscale);

                let Some(allocation) = self.sim_state.allocator.allocate(size) else {
                    return false;
                };

                self.sim_state.prepared_images.push(allocation)
            }
            InterStateImage::Animation {
                animation, load_mask, ..
            } => {
                if load_mask & self.sim_state.current_layer != 0 {
                    return true;
                }

                let Some((_, animation)) = self.animations.iter().find(|(k, _)| k == animation)
                else {
                    return true;
                };

                if animation.mode.is_from_sd_card() {
                    return true;
                }

                let size = calc_required_space(
                    animation.width,
                    animation.height,
                    animation.upscale,
                );

                for _ in 0..animation.frames.count() {
                    let Some(allocation) = self.sim_state.allocator.allocate(size) else {
                        return false;
                    };

                    self.sim_state.prepared_images.push(allocation)
                }
            }
            InterStateImage::Sequence { load_mask, mode, sequence } => {
                if load_mask & self.sim_state.current_layer != 0 {
                    return true;
                }

                let Some((_, sequence)) = self.sequences.iter()
                    .find(|(k, _)| k == sequence) else {
                    return true;
                };

                match mode {
                    SequenceMode::LoadAll => {
                        for frame in &sequence.frames {
                            let Some((_, image)) = self.images.iter()
                                .find(|(k, _)| k == &frame.image)
                            else {
                                continue;
                            };

                            let image = image.borrow();

                            let size = calc_required_space(
                                image.width,
                                image.height,
                                image.upscale,
                            );

                            let Some(allocation) = self.sim_state.allocator.allocate(size) else {
                                return false;
                            };

                            self.sim_state.prepared_images.push(allocation)
                        }
                    }
                    SequenceMode::LoadEach => {
                        let mut largest_size = 0_u64;

                        for frame in &sequence.frames {
                            let Some((_, image)) = self.images.iter().find(|(k, _)| k == &frame.image)
                            else {
                                continue;
                            };

                            let image = image.borrow();

                            let size = calc_required_space(
                                image.width,
                                image.height,
                                image.upscale,
                            );

                            if largest_size < size {
                                largest_size = size
                            }
                        }

                        for _ in 0..2 {
                            let Some(allocation) = self.sim_state.allocator.allocate(largest_size)
                            else {
                                return false;
                            };

                            self.sim_state.prepared_images.push(allocation)
                        }
                    }
                }
            },
        }

        true
    }

    pub fn prepare_layer(&mut self, new_layer: u16) -> bool {
        self.sim_state.current_layer = new_layer;

        let mut new_layer_images = HashSet::<SharedString>::new();
        let mut new_layer_anims = HashSet::<SharedString>::new();

        // Layer discovery
        for (_, state) in self.states {
            let borrowed_state = state.borrow();

            match &borrowed_state.image {
                InterStateImage::Single { load_mask, image } => {
                    if load_mask & new_layer != 0 {
                        new_layer_images.insert(image.clone());
                    }
                }
                InterStateImage::Animation { animation, load_mask, .. } => {
                    if load_mask & new_layer != 0 {
                        new_layer_anims.insert(animation.clone());
                    }
                }
                InterStateImage::Sequence { sequence, load_mask, .. } => {
                    if load_mask & new_layer != 0 {
                        let Some((_, sequence)) = self.sequences.iter()
                            .find(|(k, _)| k == sequence)
                        else {
                            continue
                        };

                        for frame in &sequence.frames {
                            new_layer_images.insert(frame.image.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // Write down unneeded images and animations
        self.sim_state.layer_images_to_remove = self.sim_state.loaded_layer_images.iter()
            .filter_map(|(k, _)| {
                if !new_layer_images.contains(k) {
                    return Some(k.clone())
                }
                None
            })
            .collect();
        self.sim_state.layer_animations_to_remove = self.sim_state.loaded_layer_animations.iter()
            .filter_map(|(k, _)| {
                if !new_layer_anims.contains(k) {
                    return Some(k.clone())
                }
                None
            })
            .collect();

        // Load missing images and animations
        for image_name in new_layer_images {
            if !self.sim_state.loaded_layer_images.contains_key(&image_name) {
                let Some((_, image)) = self.images.iter().find(|(k, _)| k == &image_name) else {
                    continue
                };

                let image = image.borrow();

                let size = calc_required_space(image.width, image.height, image.upscale);

                let Some(allocation) = self.sim_state.allocator.allocate(size) else {
                    return false;
                };

                self.sim_state.new_layer_images.insert(image_name, allocation);
            }
        }

        for anim_name in new_layer_anims {
            if !self.sim_state.loaded_layer_animations.contains_key(&anim_name) {
                let Some((_, animation)) = self.animations.iter().find(|(k, _)| k == &anim_name)
                else {
                    return true;
                };

                let size = calc_required_space(
                    animation.width,
                    animation.height,
                    animation.upscale,
                );

                let mut frames = vec![];
                for _ in 0..animation.frames.count() {
                    let Some(allocation) = self.sim_state.allocator.allocate(size) else {
                        return false;
                    };

                    frames.push(allocation)
                }

                self.sim_state.new_layer_animations.insert(anim_name, frames);
            }
        }

        true
    }

    pub fn schedule_or_switch(&mut self, state: &SharedString) -> bool {
        let Some((is_dynamic, is_layer_switch)) = self.is_dynamic_or_layer_switch(state) else {
            return true;
        };

        if is_layer_switch {
            self.sim_state.next_state = Some(state.clone());

            let Some((_, state_ref)) = self.states.iter().find(|(k, _)| k == state) else {
                println!("Couldn't find state reference!");
                return true;
            };

            if !self.prepare_layer(state_ref.borrow().layer) {
                return false;
            }

            if is_dynamic {
                if !self.cook_state(state) {
                    return false
                }
            }

            true
        } else if is_dynamic {
            self.cook_state(state)
        } else {
            self.sim_state.next_state = Some(state.clone());
            self.switch_to_scheduled();
            true
        }
    }

    pub fn find_possible_transitions(&mut self) {
        let Some((_, state)) = self
            .states
            .iter()
            .find(|(k, _)| k == &self.sim_state.current_state)
        else {
            return;
        };

        self.sim_state.possible_transitions.clear();
        self.sim_state.possible_actions.clear();

        let state = state.borrow();

        if let InterStateImage::Animation { next_state, .. } = &state.image {
            if let Some((is_dynamic, is_layer_switch)) = self.is_dynamic_or_layer_switch(next_state) {
                self.sim_state
                    .possible_transitions
                    .push(StateSwitchInfo {
                        name: next_state.clone(),
                        is_dynamic,
                        is_layer_switch,
                    });
            };
        } else {
            for transition in &state.transitions {
                let transition = transition.borrow();

                let Some((is_dynamic, is_layer_switch)) = self.is_dynamic_or_layer_switch(&transition.to_state) else {
                    continue;
                };

                self.sim_state
                    .possible_transitions
                    .push(StateSwitchInfo {
                        name: transition.to_state.clone(),
                        is_dynamic,
                        is_layer_switch,
                    });
            }

            for (_, action) in self.actions {
                let InterActionType::SwitchState(action_state) = &action.ty else {
                    continue;
                };

                let Some((is_dynamic, is_layer_switch)) = self.is_dynamic_or_layer_switch(action_state) else {
                    continue;
                };

                self.sim_state
                    .possible_actions
                    .push(StateSwitchInfo {
                        name: action_state.clone(),
                        is_dynamic,
                        is_layer_switch,
                    });
            }
        }
    }

    pub fn switch_to_scheduled(&mut self) {
        let sim = &mut *self.sim_state;

        if let Some(state) = &sim.next_state {
            sim.current_state = state.clone();
            sim.loaded_images = sim.prepared_images.clone();
            sim.prepared_images.clear();

            // Insert new images into the layer
            if !sim.new_layer_images.is_empty() {
                sim.loaded_layer_images.extend(
                    sim.new_layer_images.clone()
                );
                sim.new_layer_images.clear();
            }

            if !sim.new_layer_animations.is_empty() {
                sim.loaded_layer_animations.extend(
                    sim.new_layer_animations.clone()
                );
                sim.new_layer_animations.clear();
            }

            // Delete unneeded images
            for to_remove in &sim.layer_images_to_remove {
                sim.loaded_layer_images.remove(to_remove);
            }
            sim.layer_images_to_remove.clear();

            for to_remove in &sim.layer_animations_to_remove {
                sim.loaded_layer_animations.remove(to_remove);
            }
            sim.layer_animations_to_remove.clear();


            let Some((_, state_data)) = self.states.iter().find(|(k, _)| k == state) else {
                return;
            };

            let state_data = state_data.borrow();

            match &state_data.image {
                InterStateImage::None => {}
                InterStateImage::Single { image, load_mask } => {
                    if load_mask & sim.current_layer != 0 {
                        let Some(alloc) = sim.loaded_layer_images.get(image) else {
                            return;
                        };

                        sim.current_image = Some(alloc.clone())
                    } else {
                        sim.current_image = Some(sim.loaded_images[0].clone())
                    }
                }
                InterStateImage::Animation {
                    animation, load_mask, ..
                } => {
                    if load_mask & sim.current_layer != 0 {
                        let Some(frames) = sim.loaded_layer_animations.get(animation)
                        else {
                            return;
                        };

                        let Some(first) = frames.first() else {
                            return;
                        };

                        sim.current_image = Some(first.clone())
                    } else {
                        sim.current_image = sim.loaded_images.get(0).cloned();
                    }
                }
                InterStateImage::Sequence { load_mask, sequence, .. } => {
                    let Some((_, sequence)) = self.sequences.iter()
                        .find(|(k, _)| k == sequence) else {
                        return;
                    };

                    if load_mask & sim.current_layer != 0 {
                        let Some(first_frame) = sequence.frames.first() else {
                            return;
                        };

                        let Some(alloc) = sim.loaded_layer_images.get(&first_frame.image)
                        else {
                            return;
                        };

                        sim.current_image = Some(alloc.clone())
                    } else {
                        sim.current_image = sim.loaded_images.get(0).cloned();
                    }
                }
            }
        }

        sim.next_state = None;
        self.find_possible_transitions();
    }

    pub fn preload(&mut self) -> Option<()> {
        for (_, state) in self.states {
            let borrowed = state.borrow();

            match &borrowed.image {
                InterStateImage::None => {}
                InterStateImage::Single { image, load_mask } => {
                    if load_mask & self.sim_state.current_layer == 0 {
                        continue;
                    }

                    let Some((_, image_data)) = self.images.iter().find(|(k, _)| k == image) else {
                        continue;
                    };

                    let image_data = image_data.borrow();

                    self.sim_state.loaded_layer_images.insert(
                        image.clone(),
                        self.sim_state.allocator.allocate(calc_required_space(
                            image_data.width,
                            image_data.height,
                            image_data.upscale,
                        ))?,
                    );
                }
                InterStateImage::Animation {
                    animation, load_mask, ..
                } => {
                    if load_mask & self.sim_state.current_layer == 0 {
                        continue;
                    }

                    let Some((_, animation_info)) =
                        self.animations.iter().find(|(k, _)| k == animation)
                    else {
                        continue;
                    };

                    let frame_size = calc_required_space(
                        animation_info.width,
                        animation_info.height,
                        animation_info.upscale,
                    );

                    self.sim_state.loaded_layer_animations.insert(
                        animation.clone(),
                        (0..animation_info.frames.count())
                            .map(|_| self.sim_state.allocator.allocate(frame_size))
                            .collect::<Option<Vec<_>>>()?,
                    );
                }
                InterStateImage::Sequence { load_mask, sequence, .. } => {
                    if load_mask & self.sim_state.current_layer == 0 {
                        continue;
                    }

                    let Some((_, sequence)) = self.sequences.iter()
                        .find(|(k, _)| k == sequence) else {
                        continue;
                    };

                    for frame in &sequence.frames {
                        let Some((_, image_data)) =
                            self.images.iter().find(|(k, _)| k == &frame.image)
                        else {
                            continue;
                        };

                        let image_data = image_data.borrow();

                        self.sim_state.loaded_layer_images.insert(
                            frame.image.clone(),
                            self.sim_state.allocator.allocate(calc_required_space(
                                image_data.width,
                                image_data.height,
                                image_data.upscale,
                            ))?,
                        );
                    }
                }
            }
        }

        Some(())
    }
}

pub struct SimulatorNodeViewer<'a> {
    current_state: &'a SharedString,
    next_state: Option<&'a SharedString>,
}

impl SnarlViewer<StateNode> for SimulatorNodeViewer<'_> {
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
        let name = &snarl[node].0;

        if name == self.current_state {
            return default.fill(Color32::DARK_GREEN);
        }

        if let Some(next) = &self.next_state
            && *next == name
        {
            return default.fill(Color32::DARK_RED);
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
        _pin: &InPin,
        _ui: &mut Ui,
        _snarl: &mut Snarl<StateNode>,
    ) -> impl SnarlPin + 'static {
        PinInfo::circle()
            .with_fill(WIRE_COLOR)
            .with_wire_color(WIRE_COLOR)
    }

    fn outputs(&mut self, _node: &StateNode) -> usize {
        1
    }

    fn show_output(
        &mut self,
        _pin: &OutPin,
        _ui: &mut Ui,
        _snarl: &mut Snarl<StateNode>,
    ) -> impl SnarlPin + 'static {
        PinInfo::circle()
            .with_fill(WIRE_COLOR)
            .with_wire_color(WIRE_COLOR)
    }

    fn final_wire_shapes(
        &mut self,
        in_pins: &HashMap<InPinId, PinResponse>,
        out_pins: &HashMap<OutPinId, PinResponse>,
        shapes: &mut Vec<Shape>,
        snarl: &Snarl<StateNode>,
    ) {
        for (id, (_, state)) in snarl.node_ids() {
            let borrowed_state = state.borrow();

            match &borrowed_state.image {
                InterStateImage::Animation { next_state, .. } => {
                    if let Some((next_state_id, _)) =
                        snarl.node_ids().find(|(_, n)| &n.0 == next_state)
                    {
                        let in_pin = &in_pins[&InPinId {
                            node: next_state_id,
                            input: 0,
                        }];
                        let out_pin = &out_pins[&OutPinId {
                            node: id,
                            output: 0,
                        }];

                        shapes.extend(Shape::dashed_line(
                            &[out_pin.pos, in_pin.pos],
                            Stroke::new(3.0, WIRE_COLOR),
                            10.0,
                            10.0,
                        ))
                    }
                }
                _ => {}
            }
        }
    }

    fn final_node_rect(
        &mut self,
        node: NodeId,
        _rect: Rect,
        _ui: &mut Ui,
        snarl: &mut Snarl<StateNode>,
    ) {
        let pos = snarl.get_node_info(node).unwrap().pos;
        snarl[node].1.borrow_mut().node_pos = pos;
    }

    fn read_only(&self) -> bool {
        true
    }

    fn connect(&mut self, _: &OutPin, _: &InPin, _: &mut Snarl<StateNode>) {}

    fn disconnect(&mut self, _: &OutPin, _: &InPin, _: &mut Snarl<StateNode>) {}

    fn drop_outputs(&mut self, _: &OutPin, _: &mut Snarl<StateNode>) {}

    fn drop_inputs(&mut self, _: &InPin, _: &mut Snarl<StateNode>) {}

    fn draw_background(
        &mut self,
        background: Option<&BackgroundPattern>,
        viewport: &Rect,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
        _snarl: &Snarl<StateNode>,
    ) {
        if let Some(background) = background {
            background.draw(viewport, snarl_style, style, painter);
        }

        let stroke = Stroke::new(3.0, Color32::WHITE);
        painter.hline(-10.0..=10.0, 0.0, stroke);
        painter.vline(0.0, -10.0..=10.0, stroke);
    }
}

fn calc_required_space(width: u32, height: u32, upscale: bool) -> u64 {
    let width = if upscale { width / 2 } else { width } as u64;
    let height = if upscale { height / 2 } else { height } as u64;

    width * height * 2_u64
}

#[derive(Default)]
struct AllocationSize {
    layer_size: u64,
    new_layer_size: u64,
    loaded_size: u64,
    prepared_size: u64,
    current_size: u64,
}

fn visualize_allocator(ui: &mut Ui, state: &SimulatorState) {
    {
        const BG: Color32 = Color32::DARK_BLUE;
        const OCCUPIED: Color32 = Color32::LIGHT_BLUE;
        const CURRENT_IMAGE: Color32 = Color32::LIGHT_GREEN;
        const MARKED_LAYER_IMAGE: Color32 = Color32::DARK_GRAY;
        const LAYER_IMAGE: Color32 = Color32::GRAY;
        const NEW_LAYER_IMAGE: Color32 = Color32::DARK_RED;
        const LOADED_IMAGE: Color32 = Color32::ORANGE;
        const PREPARED_IMAGE: Color32 = Color32::LIGHT_RED;

        let (rect, _) = ui.allocate_exact_size(vec2(ui.max_rect().width(), 50.0), Sense::empty());

        let mut sizes = AllocationSize::default();

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            let inner_rect = rect.shrink(5.0);

            let text_height = 10.0;

            let total_bar_height = inner_rect.height() - text_height;
            let actual_alloc_height = (total_bar_height / 4.0).floor();
            let aware_alloc_height = total_bar_height - actual_alloc_height;
            let current_alloc_height = (aware_alloc_height / 3.0 * 2.0).floor();

            // Background
            let total_bar_rect =
                Rect::from_min_max(inner_rect.min + vec2(0.0, text_height), inner_rect.max);
            painter.rect_filled(total_bar_rect, 0, BG);

            let bytes_per_pixel = IMAGE_STORAGE_SIZE / inner_rect.width().floor() as u64;

            let paint_allocation =
                |alloc: &Allocation, y_offset: f32, height: f32, color: Color32| {
                    let start_x = (alloc.start / bytes_per_pixel) as f32;
                    let end_x = (alloc.end / bytes_per_pixel) as f32;

                    let bar_rect = Rect::from_min_size(
                        total_bar_rect.left_top() + vec2(start_x, y_offset),
                        vec2(end_x - start_x + 1.0, height),
                    );

                    painter.rect_filled(bar_rect, 0, color)
                };

            // Actual allocations / Top half of the bar
            let mut occupied_space = 0_u64;

            for allocation in state.allocator.existing_allocations() {
                occupied_space += allocation.len();
                paint_allocation(&allocation, 0.0, actual_alloc_height, OCCUPIED);
            }

            let aware_offset = actual_alloc_height;

            for (image_name, image) in &state.loaded_layer_images {
                sizes.layer_size += image.len();
                paint_allocation(
                    image.deref(),
                    aware_offset,
                    aware_alloc_height,
                    if state.layer_images_to_remove.contains(image_name) {
                        MARKED_LAYER_IMAGE
                    } else {
                        LAYER_IMAGE
                    },
                );
            }

            for (anim_name, frames) in &state.loaded_layer_animations {
                for frame in frames {
                    sizes.layer_size += frame.len();
                    paint_allocation(
                        frame.deref(),
                        aware_offset,
                        aware_alloc_height,
                        if state.layer_animations_to_remove.contains(anim_name) {
                            MARKED_LAYER_IMAGE
                        } else {
                            LAYER_IMAGE
                        },
                    );
                }
            }

            for (_, image) in &state.new_layer_images {
                sizes.new_layer_size += image.len();
                paint_allocation(
                    image.deref(),
                    aware_offset,
                    aware_alloc_height,
                    NEW_LAYER_IMAGE,
                );
            }

            for (_, frames) in &state.new_layer_animations {
                for frame in frames {
                    sizes.new_layer_size += frame.len();
                    paint_allocation(
                        frame.deref(),
                        aware_offset,
                        aware_alloc_height,
                        NEW_LAYER_IMAGE,
                    );
                }
            }

            for image in &state.loaded_images {
                sizes.loaded_size += image.len();
                paint_allocation(
                    image.deref(),
                    aware_offset,
                    aware_alloc_height,
                    LOADED_IMAGE,
                );
            }

            for image in &state.prepared_images {
                sizes.prepared_size += image.len();
                paint_allocation(
                    image.deref(),
                    aware_offset,
                    aware_alloc_height,
                    PREPARED_IMAGE,
                );
            }

            if let Some(current) = &state.current_image {
                sizes.current_size += current.len();
                paint_allocation(
                    current.deref(),
                    aware_offset,
                    current_alloc_height,
                    CURRENT_IMAGE,
                );
            }

            painter.text(
                rect.left_top(),
                Align2::LEFT_TOP,
                "Allocator State",
                FontId::proportional(10.0),
                Color32::GRAY
            );

            painter.text(
                rect.right_top(),
                Align2::RIGHT_TOP,
                format!(
                    "{}b / {}b",
                    occupied_space.to_formatted_string(&Locale::en),
                    IMAGE_STORAGE_SIZE.to_formatted_string(&Locale::en)
                ),
                FontId::proportional(10.0),
                Color32::GRAY,
            );
        }

        let draw_legend = |ui: &mut Ui, color: Color32, label: &str| {
            ui.horizontal(|ui| {
                let (resp, painter) = ui.allocate_painter(vec2(15.0, 15.0), Sense::empty());

                let inner_rect = resp.rect.shrink(2.0);

                painter.rect_filled(inner_rect, 2.5, color);

                ui.label(label);
            });
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                draw_legend(ui, OCCUPIED, "Occupied");
                draw_legend(
                    ui,
                    CURRENT_IMAGE,
                    &format!(
                        "Current ({}b)",
                        sizes.current_size.to_formatted_string(&Locale::en)
                    ),
                );
                draw_legend(
                    ui,
                    LAYER_IMAGE,
                    &format!(
                        "Loaded Layer ({}b)",
                        sizes.layer_size.to_formatted_string(&Locale::en)
                    ),
                );
                draw_legend(
                    ui,
                    NEW_LAYER_IMAGE,
                    &format!(
                        "New Layer ({}b)",
                        sizes.new_layer_size.to_formatted_string(&Locale::en)
                    ),
                );
            });

            ui.horizontal(|ui| {
                draw_legend(
                    ui,
                    LOADED_IMAGE,
                    &format!(
                        "Loaded ({}b)",
                        sizes.loaded_size.to_formatted_string(&Locale::en)
                    ),
                );
                draw_legend(
                    ui,
                    PREPARED_IMAGE,
                    &format!(
                        "Prepared ({}b)",
                        sizes.prepared_size.to_formatted_string(&Locale::en)
                    ),
                );
                draw_legend(
                    ui,
                    MARKED_LAYER_IMAGE,
                    "Marked for Delete"
                );
            });
        });
    }
}

pub struct StateSwitchInfo {
    pub name: SharedString,
    pub is_dynamic: bool,
    pub is_layer_switch: bool
}

pub struct SimulatorState {
    pub status: SimulatorStatus,
    pub allocator: AllocatorState,

    pub current_layer: u16,
    pub current_state: SharedString,
    pub next_state: Option<SharedString>,

    pub possible_transitions: Vec<StateSwitchInfo>,
    pub possible_actions: Vec<StateSwitchInfo>,

    pub current_image: Option<StrongAllocation>,

    pub new_layer_images: HashMap<SharedString, StrongAllocation>,
    pub new_layer_animations: HashMap<SharedString, Vec<StrongAllocation>>,

    pub loaded_layer_images: HashMap<SharedString, StrongAllocation>,
    pub loaded_layer_animations: HashMap<SharedString, Vec<StrongAllocation>>,

    pub layer_images_to_remove: HashSet<SharedString>,
    pub layer_animations_to_remove: HashSet<SharedString>,

    pub loaded_images: Vec<StrongAllocation>,
    pub prepared_images: Vec<StrongAllocation>,
}

#[derive(Default, EnumIs, Clone)]
pub enum SimulatorStatus {
    #[default]
    Uninitialized,
    Loaded,
    Error(String),
}

impl SimulatorState {
    pub fn error(&mut self, error: impl Display) {
        self.status = SimulatorStatus::Error(error.to_string())
    }
}

#[derive(Default)]
pub struct AllocatorState {
    allocations: Vec<WeakAllocation>,
}

pub const IMAGE_STORAGE_SIZE: u64 = 7_000_000;

impl AllocatorState {
    fn clear_expired(&mut self) {
        self.allocations.retain(|e| e.upgrade().is_some())
    }

    pub fn existing_allocations(&self) -> Vec<Allocation> {
        self.allocations
            .iter()
            .filter_map(|e| e.upgrade())
            .map(|e| e.deref().clone())
            .collect()
    }

    fn find_space(&self, size: u64) -> Option<u64> {
        let mut existing = self.existing_allocations();

        existing.sort_unstable();

        let mut block_start = 0_u64;

        for occlusion in &existing {
            let current_size = occlusion.start - block_start;

            if current_size >= size {
                return Some(block_start);
            }

            block_start = occlusion.end + 1
        }

        let remaining_size = IMAGE_STORAGE_SIZE - block_start;

        if remaining_size >= size {
            return Some(block_start);
        }

        None
    }

    pub fn allocate(&mut self, size: u64) -> Option<StrongAllocation> {
        self.clear_expired();

        let block_start = self.find_space(size)?;

        let ptr = Rc::new(Allocation {
            start: block_start,
            end: block_start + size - 1,
        });

        self.allocations.push(Rc::downgrade(&ptr));

        Some(ptr)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord)]
pub struct Allocation {
    pub start: u64,
    pub end: u64,
}

pub type StrongAllocation = Rc<Allocation>;
pub type WeakAllocation = Weak<Allocation>;

impl PartialOrd for Allocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.end < other.start {
            return Some(Ordering::Less);
        }

        if other.end < self.start {
            return Some(Ordering::Greater);
        }

        Some(Ordering::Equal)
    }
}

impl Allocation {
    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }
}
