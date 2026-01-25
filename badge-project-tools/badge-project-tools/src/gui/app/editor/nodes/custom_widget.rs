use std::collections::HashMap;
use std::hash::Hash;
use eframe::emath::{Rect, Vec2};
use eframe::epaint::{Shape, Stroke, StrokeKind};
use egui::{Id, LayerId, PointerButton, Scene, Sense, Ui, UiBuilder, UiKind, UiStackInfo};
use egui::response::Flags;
use egui_snarl::{InPin, OutPin, Snarl};
use egui_snarl::ui::{AnyPins, SnarlStyle, SnarlViewer, WireLayer};

/// Widget to display [`Snarl`] graph in [`Ui`].
#[derive(Clone, Copy, Debug)]
pub struct MySnarlWidget {
    id_salt: Id,
    id: Option<Id>,
    style: SnarlStyle,
    min_size: Vec2,
    max_size: Vec2,
}

impl Default for MySnarlWidget {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl MySnarlWidget {
    /// Returns new [`egui_snarl::ui::SnarlWidget`] with default parameters.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            id_salt: Id::new(":snarl:"),
            id: None,
            style: SnarlStyle::new(),
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
        }
    }

    /// Assign an explicit and globally unique [`Id`].
    ///
    /// Use this if you want to persist the state of the widget
    /// when it changes position in the widget hierarchy.
    ///
    /// Prefer using [`egui_snarl::ui::SnarlWidget::id_salt`] otherwise.
    #[inline]
    #[must_use]
    pub const fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Assign a source for the unique [`Id`]
    ///
    /// It must be locally unique for the current [`Ui`] hierarchy position.
    ///
    /// Ignored if [`egui_snarl::ui::SnarlWidget::id`] was set.
    #[inline]
    #[must_use]
    pub fn id_salt(mut self, id_salt: impl Hash) -> Self {
        self.id_salt = Id::new(id_salt);
        self
    }

    /// Set style parameters for the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub const fn style(mut self, style: SnarlStyle) -> Self {
        self.style = style;
        self
    }

    /// Set minimum size of the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub const fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set maximum size of the [`Snarl`] widget.
    #[inline]
    #[must_use]
    pub const fn max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = max_size;
        self
    }

    #[inline]
    fn get_id(&self, ui_id: Id) -> Id {
        self.id.unwrap_or_else(|| ui_id.with(self.id_salt))
    }

    /// Render [`Snarl`] using given viewer and style into the [`Ui`].
    #[inline]
    pub fn show<T, V>(&self, snarl: &mut Snarl<T>, viewer: &mut V, ui: &mut Ui) -> egui::Response
    where
        V: SnarlViewer<T>,
    {
        let snarl_id = self.get_id(ui.id());

        show_snarl(
            snarl_id,
            self.style,
            self.min_size,
            self.max_size,
            snarl,
            viewer,
            ui,
        )
    }
}

#[inline(never)]
fn show_snarl<T, V>(
    snarl_id: Id,
    mut style: SnarlStyle,
    min_size: Vec2,
    max_size: Vec2,
    snarl: &mut Snarl<T>,
    viewer: &mut V,
    ui: &mut Ui,
) -> egui::Response
where
    V: SnarlViewer<T>,
{
    #![allow(clippy::too_many_lines)]

    let (mut latest_pos, modifiers) = ui.ctx().input(|i| (i.pointer.latest_pos(), i.modifiers));

    let bg_frame = style.get_bg_frame(ui.style());

    let outer_size_bounds = ui.available_size_before_wrap().max(min_size).min(max_size);

    let outer_resp = ui.allocate_response(outer_size_bounds, Sense::hover());

    ui.painter().add(bg_frame.paint(outer_resp.rect));

    let mut content_rect = outer_resp.rect - bg_frame.total_margin();

    // Make sure we don't shrink to the negative:
    content_rect.max.x = content_rect.max.x.max(content_rect.min.x);
    content_rect.max.y = content_rect.max.y.max(content_rect.min.y);

    let snarl_layer_id = LayerId::new(ui.layer_id().order, snarl_id);

    ui.ctx().set_sublayer(ui.layer_id(), snarl_layer_id);

    let mut min_scale = style.get_min_scale();
    let mut max_scale = style.get_max_scale();

    let ui_rect = content_rect;

    let mut snarl_state =
        SnarlState::load(ui.ctx(), snarl_id, snarl, ui_rect, min_scale, max_scale);
    let mut to_global = snarl_state.to_global();

    let clip_rect = ui.clip_rect();

    let mut ui = ui.new_child(
        UiBuilder::new()
            .ui_stack_info(UiStackInfo::new(UiKind::Frame).with_frame(bg_frame))
            .layer_id(snarl_layer_id)
            .max_rect(Rect::EVERYTHING)
            .sense(Sense::click_and_drag()),
    );

    if style.get_crisp_magnified_text() {
        style.scale(max_scale);
        ui.style_mut().scale(max_scale);

        min_scale /= max_scale;
        max_scale = 1.0;
    }

    clamp_scale(&mut to_global, min_scale, max_scale, ui_rect);

    let mut snarl_resp = ui.response();
    Scene::new()
        .zoom_range(min_scale..=max_scale)
        .register_pan_and_zoom(&ui, &mut snarl_resp, &mut to_global);

    if snarl_resp.changed() {
        ui.ctx().request_repaint();
    }

    // Inform viewer about current transform.
    viewer.current_transform(&mut to_global, snarl);

    snarl_state.set_to_global(to_global);

    let to_global = to_global;
    let from_global = to_global.inverse();

    // Graph viewport
    let viewport = (from_global * ui_rect).round_ui();
    let viewport_clip = from_global * clip_rect;

    ui.set_clip_rect(viewport.intersect(viewport_clip));
    ui.expand_to_include_rect(viewport);

    // Set transform for snarl layer.
    ui.ctx().set_transform_layer(snarl_layer_id, to_global);

    // Map latest pointer position to graph space.
    latest_pos = latest_pos.map(|pos| from_global * pos);

    viewer.draw_background(
        style.bg_pattern.as_ref(),
        &viewport,
        &style,
        ui.style(),
        ui.painter(),
        snarl,
    );

    let mut node_moved = None;
    let mut node_to_top = None;

    // Process selection rect.
    let mut rect_selection_ended = None;
    if modifiers.shift || snarl_state.is_rect_selection() {
        let select_resp = ui.interact(snarl_resp.rect, snarl_id.with("select"), Sense::drag());

        if select_resp.dragged_by(PointerButton::Primary)
            && let Some(pos) = select_resp.interact_pointer_pos()
        {
            if snarl_state.is_rect_selection() {
                snarl_state.update_rect_selection(pos);
            } else {
                snarl_state.start_rect_selection(pos);
            }
        }

        if select_resp.drag_stopped_by(PointerButton::Primary) {
            if let Some(select_rect) = snarl_state.rect_selection() {
                rect_selection_ended = Some(select_rect);
            }
            snarl_state.stop_rect_selection();
        }
    }

    let wire_frame_size = style.get_wire_frame_size(ui.style());
    let wire_width = style.get_wire_width(ui.style());
    let wire_threshold = style.get_wire_smoothness();

    let wire_shape_idx = match style.get_wire_layer() {
        WireLayer::BehindNodes => Some(ui.painter().add(Shape::Noop)),
        WireLayer::AboveNodes => None,
    };

    let mut input_info = HashMap::new();
    let mut output_info = HashMap::new();

    let mut pin_hovered = None;

    let draw_order = snarl_state.update_draw_order(snarl);
    let mut drag_released = false;

    let mut nodes_bb = Rect::NOTHING;
    let mut node_rects = Vec::new();

    for node_idx in draw_order {
        if !snarl.nodes.contains(node_idx.0) {
            continue;
        }

        // show_node(node_idx);
        let response = draw_node(
            snarl,
            &mut ui,
            node_idx,
            viewer,
            &mut snarl_state,
            &style,
            snarl_id,
            &mut input_info,
            modifiers,
            &mut output_info,
        );

        if let Some(response) = response {
            if let Some(v) = response.node_to_top {
                node_to_top = Some(v);
            }
            if let Some(v) = response.node_moved {
                node_moved = Some(v);
            }
            if let Some(v) = response.pin_hovered {
                pin_hovered = Some(v);
            }
            drag_released |= response.drag_released;

            nodes_bb = nodes_bb.union(response.final_rect);
            if rect_selection_ended.is_some() {
                node_rects.push((node_idx, response.final_rect));
            }
        }
    }

    let mut hovered_wire = None;
    let mut hovered_wire_disconnect = false;
    let mut wire_shapes = Vec::new();

    // Draw and interact with wires
    for wire in snarl.wires.iter() {
        let Some(from_r) = output_info.get(&wire.out_pin) else {
            continue;
        };
        let Some(to_r) = input_info.get(&wire.in_pin) else {
            continue;
        };

        if !snarl_state.has_new_wires() && snarl_resp.contains_pointer() && hovered_wire.is_none() {
            // Try to find hovered wire
            // If not dragging new wire
            // And not hovering over item above.

            if let Some(latest_pos) = latest_pos {
                let wire_hit = hit_wire(
                    ui.ctx(),
                    WireId::Connected {
                        snarl_id,
                        out_pin: wire.out_pin,
                        in_pin: wire.in_pin,
                    },
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_r.pos,
                    to_r.pos,
                    latest_pos,
                    wire_width.max(2.0),
                    pick_wire_style(from_r.wire_style, to_r.wire_style),
                );

                if wire_hit {
                    hovered_wire = Some(wire);

                    let wire_r =
                        ui.interact(snarl_resp.rect, ui.make_persistent_id(wire), Sense::click());

                    //Remove hovered wire by second click
                    hovered_wire_disconnect |= wire_r.clicked_by(PointerButton::Secondary);
                }
            }
        }

        let color = mix_colors(from_r.wire_color, to_r.wire_color);

        let mut draw_width = wire_width;
        if hovered_wire == Some(wire) {
            draw_width *= 1.5;
        }

        draw_wire(
            &ui,
            WireId::Connected {
                snarl_id,
                out_pin: wire.out_pin,
                in_pin: wire.in_pin,
            },
            &mut wire_shapes,
            wire_frame_size,
            style.get_upscale_wire_frame(),
            style.get_downscale_wire_frame(),
            from_r.pos,
            to_r.pos,
            Stroke::new(draw_width, color),
            wire_threshold,
            pick_wire_style(from_r.wire_style, to_r.wire_style),
        );
    }

    // Remove hovered wire by second click
    if hovered_wire_disconnect && let Some(wire) = hovered_wire {
        let out_pin = OutPin::new(snarl, wire.out_pin);
        let in_pin = InPin::new(snarl, wire.in_pin);
        viewer.disconnect(&out_pin, &in_pin, snarl);
    }

    if let Some(select_rect) = rect_selection_ended {
        let select_nodes = node_rects.into_iter().filter_map(|(id, rect)| {
            let select = if style.get_select_rect_contained() {
                select_rect.contains_rect(rect)
            } else {
                select_rect.intersects(rect)
            };

            if select { Some(id) } else { None }
        });

        if modifiers.command {
            snarl_state.deselect_many_nodes(select_nodes);
        } else {
            snarl_state.select_many_nodes(!modifiers.shift, select_nodes);
        }
    }

    if let Some(select_rect) = snarl_state.rect_selection() {
        ui.painter().rect(
            select_rect,
            0.0,
            style.get_select_fill(ui.style()),
            style.get_select_stroke(ui.style()),
            StrokeKind::Inside,
        );
    }

    // If right button is clicked while new wire is being dragged, cancel it.
    // This is to provide way to 'not open' the link graph node menu, but just
    // releasing the new wire to empty space.
    //
    // This uses `button_down` directly, instead of `clicked_by` to improve
    // responsiveness of the cancel action.
    if snarl_state.has_new_wires() && ui.input(|x| x.pointer.button_down(PointerButton::Secondary))
    {
        let _ = snarl_state.take_new_wires();
        snarl_resp.flags.remove(Flags::CLICKED);
    }

    // Do centering unless no nodes are present.
    if style.get_centering() && snarl_resp.double_clicked() && nodes_bb.is_finite() {
        let nodes_bb = nodes_bb.expand(100.0);
        snarl_state.look_at(nodes_bb, ui_rect, min_scale, max_scale);
    }

    if modifiers.command && snarl_resp.clicked_by(PointerButton::Primary) {
        snarl_state.deselect_all_nodes();
    }

    // Wire end position will be overridden when link graph menu is opened.
    let mut wire_end_pos = latest_pos.unwrap_or_else(|| snarl_resp.rect.center());

    if drag_released {
        let new_wires = snarl_state.take_new_wires();
        if new_wires.is_some() {
            ui.ctx().request_repaint();
        }
        match (new_wires, pin_hovered) {
            (Some(NewWires::In(in_pins)), Some(AnyPin::Out(out_pin))) => {
                for in_pin in in_pins {
                    viewer.connect(
                        &OutPin::new(snarl, out_pin),
                        &InPin::new(snarl, in_pin),
                        snarl,
                    );
                }
            }
            (Some(NewWires::Out(out_pins)), Some(AnyPin::In(in_pin))) => {
                for out_pin in out_pins {
                    viewer.connect(
                        &OutPin::new(snarl, out_pin),
                        &InPin::new(snarl, in_pin),
                        snarl,
                    );
                }
            }
            (Some(new_wires), None) if snarl_resp.hovered() => {
                let pins = match &new_wires {
                    NewWires::In(x) => AnyPins::In(x),
                    NewWires::Out(x) => AnyPins::Out(x),
                };

                if viewer.has_dropped_wire_menu(pins, snarl) {
                    // A wire is dropped without connecting to a pin.
                    // Show context menu for the wire drop.
                    snarl_state.set_new_wires_menu(new_wires);

                    // Force open context menu.
                    snarl_resp.flags.insert(Flags::LONG_TOUCHED);
                }
            }
            _ => {}
        }
    }

    if let Some(interact_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
        if let Some(new_wires) = snarl_state.take_new_wires_menu() {
            let pins = match &new_wires {
                NewWires::In(x) => AnyPins::In(x),
                NewWires::Out(x) => AnyPins::Out(x),
            };

            if viewer.has_dropped_wire_menu(pins, snarl) {
                snarl_resp.context_menu(|ui| {
                    let pins = match &new_wires {
                        NewWires::In(x) => AnyPins::In(x),
                        NewWires::Out(x) => AnyPins::Out(x),
                    };

                    let menu_pos = from_global * ui.cursor().min;

                    // Override wire end position when the wire-drop context menu is opened.
                    wire_end_pos = menu_pos;

                    // The context menu is opened as *link* graph menu.
                    viewer.show_dropped_wire_menu(menu_pos, ui, pins, snarl);

                    // Even though menu could be closed in `show_dropped_wire_menu`,
                    // we need to revert the new wires here, because menu state is inaccessible.
                    // Next frame context menu won't be shown and wires will be removed.
                    snarl_state.set_new_wires_menu(new_wires);
                });
            }
        } else if viewer.has_graph_menu(interact_pos, snarl) {
            snarl_resp.context_menu(|ui| {
                let menu_pos = from_global * ui.cursor().min;

                viewer.show_graph_menu(menu_pos, ui, snarl);
            });
        }
    }

    match snarl_state.new_wires() {
        None => {}
        Some(NewWires::In(in_pins)) => {
            for &in_pin in in_pins {
                let from_pos = wire_end_pos;
                let to_r = &input_info[&in_pin];

                draw_wire(
                    &ui,
                    WireId::NewInput { snarl_id, in_pin },
                    &mut wire_shapes,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_pos,
                    to_r.pos,
                    Stroke::new(wire_width, to_r.wire_color),
                    wire_threshold,
                    to_r.wire_style,
                );
            }
        }
        Some(NewWires::Out(out_pins)) => {
            for &out_pin in out_pins {
                let from_r = &output_info[&out_pin];
                let to_pos = wire_end_pos;

                draw_wire(
                    &ui,
                    WireId::NewOutput { snarl_id, out_pin },
                    &mut wire_shapes,
                    wire_frame_size,
                    style.get_upscale_wire_frame(),
                    style.get_downscale_wire_frame(),
                    from_r.pos,
                    to_pos,
                    Stroke::new(wire_width, from_r.wire_color),
                    wire_threshold,
                    from_r.wire_style,
                );
            }
        }
    }

    match wire_shape_idx {
        None => {
            ui.painter().add(Shape::Vec(wire_shapes));
        }
        Some(idx) => {
            ui.painter().set(idx, Shape::Vec(wire_shapes));
        }
    }

    ui.advance_cursor_after_rect(Rect::from_min_size(snarl_resp.rect.min, Vec2::ZERO));

    if let Some(node) = node_to_top
        && snarl.nodes.contains(node.0)
    {
        snarl_state.node_to_top(node);
    }

    if let Some((node, delta)) = node_moved
        && snarl.nodes.contains(node.0)
    {
        ui.ctx().request_repaint();
        if snarl_state.selected_nodes().contains(&node) {
            for node in snarl_state.selected_nodes() {
                let node = &mut snarl.nodes[node.0];
                node.pos += delta;
            }
        } else {
            let node = &mut snarl.nodes[node.0];
            node.pos += delta;
        }
    }

    snarl_state.store(snarl, ui.ctx());

    snarl_resp
}