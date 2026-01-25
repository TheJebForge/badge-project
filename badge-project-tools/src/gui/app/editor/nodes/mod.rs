mod custom_widget;

use std::collections::HashMap;
use eframe::emath::Rect;
use egui::{vec2, CentralPanel, Color32, Frame, Id, Painter, Sense, Stroke, Style, Ui};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use egui_snarl::ui::{BackgroundPattern, Grid, PinInfo, PinPlacement, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget, WireLayer};
use strum::EnumIs;
use crate::gui::app::editor::CharacterEditor;
use crate::gui::app::editor::intermediate::{SharedInterState, SharedInterStateTransition};
use crate::gui::app::shared::SharedString;

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
        wire_layer: Some(WireLayer::AboveNodes),
        wire_width: Some(3.0),
        header_frame: Some(
            Frame::new()
        ),
        ..SnarlStyle::new()
    }
}

pub fn snarl_from_states(states: &Vec<(SharedString, SharedInterState)>) -> Snarl<StateNode> {
    let mut snarl = Snarl::new();

    // Create nodes
    let mut mapping: HashMap<SharedString, NodeId> = HashMap::new();

    for (name, state) in states {
        let id = snarl.insert_node(
            state.borrow().initial_node_pos.clone(),
            (name.clone(), state.clone())
        );

        mapping.insert(name.clone(), id);
    }

    // Create connections
    for (this_name, state) in states {
        let borrowed_state = state.borrow();

        let Some(this_node) = mapping.get(this_name) else {
            println!("failed to get lhs mapping for {this_name}");
            continue
        };

        let out_pin = OutPinId {
            node: this_node.clone(),
            output: 0,
        };

        for transition in &borrowed_state.transitions {
            let borrowed_transition = transition.borrow();

            let Some(other_node) = mapping.get(&borrowed_transition.to_state) else {
                println!("failed to get rhs mapping for {}", borrowed_transition.to_state);
                continue
            };

            let in_pin = InPinId {
                node: other_node.clone(),
                input: 0
            };

            snarl.connect(out_pin.clone(), in_pin);
        }
    }

    snarl
}

#[derive(EnumIs, Default)]
enum ViewerSelection {
    #[default]
    None,
    SelectedState {
        state: SharedInterState,
        node: NodeId
    },
    SelectedTransition {
        transition: SharedInterStateTransition,
        out_pin_id: OutPinId,
        in_pin_id: InPinId
    }
}

#[derive(Default)]
pub struct StateViewer {
    selection: ViewerSelection
}

const WIRE_COLOR: Color32 = Color32::LIGHT_GRAY;

impl SnarlViewer<StateNode> for StateViewer {
    fn title(&mut self, node: &StateNode) -> String {
        node.0.to_string()
    }

    fn show_header(&mut self, node: NodeId, inputs: &[InPin], outputs: &[OutPin], ui: &mut Ui, snarl: &mut Snarl<StateNode>) {
        ui.label(self.title(&snarl[node]));
        ui.add_space(4.0);
    }

    fn inputs(&mut self, node: &StateNode) -> usize {
        1
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<StateNode>) -> impl SnarlPin + 'static {
        PinInfo::circle()
            .with_fill(WIRE_COLOR)
            .with_wire_color(WIRE_COLOR)
    }

    fn outputs(&mut self, node: &StateNode) -> usize {
        1
    }

    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<StateNode>) -> impl SnarlPin + 'static {
        PinInfo::circle()
            .with_fill(WIRE_COLOR)
            .with_wire_color(WIRE_COLOR)
    }

    fn final_node_rect(&mut self, node: NodeId, rect: Rect, ui: &mut Ui, snarl: &mut Snarl<StateNode>) {
        let resp = ui.allocate_rect(rect, Sense::CLICK);

        if resp.secondary_clicked() {
            println!("wah {}", snarl[node].0)
        }
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<StateNode>) {
        println!("right click");
    }
}

impl CharacterEditor {
    pub(crate) fn state_machine_ui(&mut self, ui: &mut Ui) {
        CentralPanel::default()
            .show(ui.ctx(), |ui| {
                SnarlWidget::new()
                    .id(Id::new("state_machine.graph"))
                    .style(self.graph_style)
                    .show(&mut self.state_graph, &mut self.graph_viewer, ui);

            });
    }
}