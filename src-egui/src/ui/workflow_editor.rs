use crate::models::{Workflow, WorkflowNodeType, WorkflowTask};
use egui::{Color32, Context, Pos2, Rect, RichText, Stroke, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

pub struct WorkflowEditorView {
    workflow: Option<Workflow>,
    selected_node: Option<Uuid>,
    dragging_node: Option<Uuid>,
    drag_offset: Vec2,
    canvas_offset: Vec2,
    zoom: f32,
    connection_start: Option<Uuid>,
    node_positions: HashMap<Uuid, Pos2>,
}

impl WorkflowEditorView {
    pub fn new() -> Self {
        Self {
            workflow: None,
            selected_node: None,
            dragging_node: None,
            drag_offset: Vec2::ZERO,
            canvas_offset: Vec2::ZERO,
            zoom: 1.0,
            connection_start: None,
            node_positions: HashMap::new(),
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        // Top toolbar
        egui::TopBottomPanel::top("workflow_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📄 New Workflow").clicked() {
                    self.workflow = Some(Workflow::new(
                        "New Workflow".to_string(),
                        "Description".to_string(),
                    ));
                }
                
                ui.separator();
                
                if self.workflow.is_some() {
                    if ui.button("💾 Save").clicked() {
                        // TODO: Save workflow
                    }
                    
                    if ui.button("📂 Load").clicked() {
                        // TODO: Load workflow
                    }
                    
                    ui.separator();
                    
                    if ui.button("▶ Execute").clicked() {
                        // TODO: Execute workflow
                    }
                    
                    if ui.button("✓ Validate").clicked() {
                        // TODO: Validate workflow
                    }
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
                    if ui.button("🔍+").clicked() {
                        self.zoom = (self.zoom * 1.1).min(3.0);
                    }
                    if ui.button("🔍-").clicked() {
                        self.zoom = (self.zoom / 1.1).max(0.3);
                    }
                    if ui.button("🔍⟲").clicked() {
                        self.zoom = 1.0;
                        self.canvas_offset = Vec2::ZERO;
                    }
                });
            });
        });

        // Left panel - Node palette
        egui::SidePanel::left("node_palette")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.show_node_palette(ui);
            });

        // Right panel - Properties
        if self.selected_node.is_some() {
            egui::SidePanel::right("properties_panel")
                .default_width(300.0)
                .show(ctx, |ui| {
                    self.show_properties_panel(ui);
                });
        }

        // Central canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_canvas(ui);
        });
    }

    fn show_node_palette(&self, ui: &mut egui::Ui) {
        ui.heading("Node Types");
        ui.separator();
        
        ui.label("Drag to canvas:");
        ui.add_space(10.0);
        
        // Task node
        let task_response = ui.add(
            egui::Button::new("📋 Task")
                .min_size(Vec2::new(150.0, 40.0))
        );
        if task_response.drag_started() {
            // TODO: Start dragging new task node
        }
        
        // Parallel node
        let _parallel_response = ui.add(
            egui::Button::new("⚡ Parallel")
                .min_size(Vec2::new(150.0, 40.0))
        );
        
        // Conditional node
        let _conditional_response = ui.add(
            egui::Button::new("❓ Conditional")
                .min_size(Vec2::new(150.0, 40.0))
        );
        
        // Loop node
        let _loop_response = ui.add(
            egui::Button::new("🔄 Loop")
                .min_size(Vec2::new(150.0, 40.0))
        );
        
        // Wait node
        let _wait_response = ui.add(
            egui::Button::new("⏱ Wait")
                .min_size(Vec2::new(150.0, 40.0))
        );
        
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        
        ui.heading("Templates");
        ui.add_space(10.0);
        
        if ui.button("Web App Development").clicked() {
            // TODO: Load template
        }
        
        if ui.button("Code Review Pipeline").clicked() {
            // TODO: Load template
        }
        
        if ui.button("Data Processing").clicked() {
            // TODO: Load template
        }
    }

    fn show_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Node Properties");
        ui.separator();
        
        if let Some(node_id) = self.selected_node {
            // Extract the node data we need
            let mut node_data = None;
            if let Some(workflow) = &mut self.workflow {
                if let Some(node) = workflow.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                    node_data = Some((node.name.clone(), node.node_type.clone()));
                }
            }
            
            if let Some((mut name, node_type)) = node_data {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut name).changed() {
                        // Update the name in the workflow
                        if let Some(workflow) = &mut self.workflow {
                            if let Some(node) = workflow.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                                node.name = name;
                            }
                        }
                    }
                });
                
                ui.add_space(10.0);
                
                match node_type {
                    WorkflowNodeType::Task(mut task) => {
                        self.show_task_properties(ui, &mut task);
                        // Update task in workflow if changed
                        if let Some(workflow) = &mut self.workflow {
                            if let Some(node) = workflow.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                                if let WorkflowNodeType::Task(ref mut t) = node.node_type {
                                    *t = task;
                                }
                            }
                        }
                    }
                    WorkflowNodeType::Conditional { mut condition, .. } => {
                        ui.label("Condition:");
                        if ui.text_edit_singleline(&mut condition).changed() {
                            // Update condition in workflow
                            if let Some(workflow) = &mut self.workflow {
                                if let Some(node) = workflow.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                                    if let WorkflowNodeType::Conditional { condition: c, .. } = &mut node.node_type {
                                        *c = condition;
                                    }
                                }
                            }
                        }
                    }
                    WorkflowNodeType::Loop { mut condition, mut max_iterations, .. } => {
                        ui.label("Loop Condition:");
                        let cond_changed = ui.text_edit_singleline(&mut condition).changed();
                        
                        let mut iter_changed = false;
                        ui.horizontal(|ui| {
                            ui.label("Max Iterations:");
                            if let Some(max) = &mut max_iterations {
                                iter_changed = ui.add(egui::DragValue::new(max)).changed();
                            }
                        });
                        
                        if cond_changed || iter_changed {
                            // Update in workflow
                            if let Some(workflow) = &mut self.workflow {
                                if let Some(node) = workflow.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                                    if let WorkflowNodeType::Loop { condition: c, max_iterations: m, .. } = &mut node.node_type {
                                        *c = condition;
                                        *m = max_iterations;
                                    }
                                }
                            }
                        }
                    }
                    WorkflowNodeType::Wait { duration } => {
                        ui.label("Wait Duration:");
                        ui.label(format!("{} seconds", duration.num_seconds()));
                    }
                    _ => {}
                }
            }
        }
    }

    fn show_task_properties(&self, ui: &mut egui::Ui, task: &mut WorkflowTask) {
        ui.label("Description:");
        ui.text_edit_multiline(&mut task.description);
        
        ui.add_space(10.0);
        
        ui.label("Required Agent Role:");
        // TODO: Add role selector
        
        ui.add_space(10.0);
        
        ui.collapsing("Inputs", |ui| {
            for input in &task.inputs {
                ui.label(&input.name);
            }
            if ui.button("+ Add Input").clicked() {
                // TODO: Add input
            }
        });
        
        ui.collapsing("Outputs", |ui| {
            for output in &task.outputs {
                ui.label(&output.name);
            }
            if ui.button("+ Add Output").clicked() {
                // TODO: Add output
            }
        });
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let canvas_rect = ui.available_rect_before_wrap();
        
        // Handle canvas pan
        let response = ui.interact(canvas_rect, ui.id(), egui::Sense::drag());
        if response.dragged_by(egui::PointerButton::Middle) {
            self.canvas_offset += response.drag_delta();
        }
        
        // Handle zoom
        if ui.input(|i| i.raw_scroll_delta.y != 0.0) {
            let zoom_delta = ui.input(|i| i.raw_scroll_delta.y * 0.001);
            self.zoom = (self.zoom * (1.0 + zoom_delta)).clamp(0.3, 3.0);
        }
        
        if self.workflow.is_some() {
            // Draw grid
            self.draw_grid(ui, canvas_rect);
            
            // Draw connections
            let workflow_clone = self.workflow.clone();
            if let Some(workflow) = &workflow_clone {
                self.draw_connections(ui, workflow);
            }
            
            // Draw nodes
            self.draw_nodes(ui);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Create or load a workflow to begin").size(20.0));
            });
        }
    }

    fn draw_grid(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        let grid_size = 20.0 * self.zoom;
        
        let offset_x = self.canvas_offset.x % grid_size;
        let offset_y = self.canvas_offset.y % grid_size;
        
        // Draw vertical lines
        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5, Color32::from_gray(40)),
            );
            x += grid_size;
        }
        
        // Draw horizontal lines
        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5, Color32::from_gray(40)),
            );
            y += grid_size;
        }
    }

    fn draw_connections(&self, _ui: &mut egui::Ui, _workflow: &Workflow) {
        // TODO: Draw bezier curves between connected nodes
    }

    fn draw_nodes(&mut self, ui: &mut egui::Ui) {
        if let Some(workflow) = &self.workflow {
            let nodes = workflow.graph.nodes.clone();
            for node in &nodes {
            let pos = self.node_positions.get(&node.id)
                .copied()
                .unwrap_or(Pos2::new(node.position.0, node.position.1));
            
            let screen_pos = pos * self.zoom + self.canvas_offset;
            let node_size = Vec2::new(150.0, 80.0) * self.zoom;
            let node_rect = Rect::from_min_size(screen_pos, node_size);
            
            // Node background
            let is_selected = self.selected_node == Some(node.id);
            let fill_color = if is_selected {
                ui.style().visuals.selection.bg_fill
            } else {
                match &node.node_type {
                    WorkflowNodeType::Start => Color32::from_rgb(50, 150, 50),
                    WorkflowNodeType::End => Color32::from_rgb(150, 50, 50),
                    WorkflowNodeType::Task(_) => Color32::from_rgb(50, 50, 150),
                    WorkflowNodeType::Parallel(_) => Color32::from_rgb(150, 150, 50),
                    WorkflowNodeType::Conditional { .. } => Color32::from_rgb(150, 50, 150),
                    WorkflowNodeType::Loop { .. } => Color32::from_rgb(50, 150, 150),
                    WorkflowNodeType::Wait { .. } => Color32::from_rgb(100, 100, 100),
                }
            };
            
            ui.painter().rect(
                node_rect,
                5.0,
                fill_color,
                Stroke::new(2.0, Color32::WHITE),
            );
            
            // Node icon and text
            let icon = match &node.node_type {
                WorkflowNodeType::Start => "▶",
                WorkflowNodeType::End => "■",
                WorkflowNodeType::Task(_) => "📋",
                WorkflowNodeType::Parallel(_) => "⚡",
                WorkflowNodeType::Conditional { .. } => "❓",
                WorkflowNodeType::Loop { .. } => "🔄",
                WorkflowNodeType::Wait { .. } => "⏱",
            };
            
            ui.painter().text(
                node_rect.center() - Vec2::new(0.0, 10.0 * self.zoom),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(20.0 * self.zoom),
                Color32::WHITE,
            );
            
            ui.painter().text(
                node_rect.center() + Vec2::new(0.0, 15.0 * self.zoom),
                egui::Align2::CENTER_CENTER,
                &node.name,
                egui::FontId::proportional(12.0 * self.zoom),
                Color32::WHITE,
            );
            
            // Handle interactions
            let response = ui.interact(node_rect, ui.id().with(node.id), egui::Sense::click_and_drag());
            
            if response.clicked() {
                self.selected_node = Some(node.id);
            }
            
            if response.drag_started() {
                self.dragging_node = Some(node.id);
                self.drag_offset = screen_pos - response.interact_pointer_pos().unwrap_or(screen_pos);
            }
            
            if response.dragged() && self.dragging_node == Some(node.id) {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let new_pos = (pointer_pos + self.drag_offset - self.canvas_offset) / self.zoom;
                    self.node_positions.insert(node.id, new_pos);
                }
            }
            
            if response.drag_stopped() {
                self.dragging_node = None;
            }
            }
        }
    }
}