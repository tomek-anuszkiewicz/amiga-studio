//! Per-frame GUI updates and presentation.

use super::{EmulatorApp, LayoutTier, ViewMode};
use crate::host_commands::HostCommand;
use crate::host_video::VideoRead;
use crate::layout::left_dock::disassembly::render_disassembly;
use crate::layout::left_dock::engine_status::render_engine_status;
use crate::layout::left_dock::microcode::render_microcode;
use crate::layout::left_dock::registers::render_registers;
use crate::layout::main_viewport::amiga_screen::{render_amiga_screen, ScreenTexture};
use crate::layout::main_viewport::temporal_bar::render_temporal_bar;
use crate::layout::right_dock::breakpoints_panel::render_breakpoints_panel;
use crate::layout::right_dock::memory_hex::render_memory_hex;
use crate::layout::right_dock::memory_search::render_memory_search;
use crate::layout::right_dock::trace_log::render_trace_log;
use crate::layout::top_menu_bar::render_top_menu_bar;

impl EmulatorApp {
    /// Renders one complete UI frame using owner replies, image notifications, and docked panels
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let ctx = &context;
        let first_pass = ctx.output(|output| output.num_completed_passes == 0);
        crate::keyboard_input::begin_editor_collection(ctx);
        self.poll_host(ctx, first_pass);
        self.poll_configuration_save(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        if first_pass {
            self.poll_library(ctx);
            self.finish_start_state_deletions(false);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.render_start_state_undo(ui);

        self.screen_texture.set_scale(self.screen_scale);
        let consume_image = first_pass && self.host.error.is_none();
        let mut next_image = None;
        if let Some(runtime) = self.host.runtime.as_mut().filter(|_| consume_image) {
            let result = runtime.try_consume(|info, status, pixels| {
                if !status.running && info.cck != status.cck {
                    return None;
                }
                Some((info, ScreenTexture::convert(info, pixels)))
            });
            let status = runtime.status();
            let previous = self.host.status;
            self.host.accept_status(status);
            if previous != status {
                if previous.generation != status.generation {
                    self.player.captured = false;
                    if self.host.data.is_none() {
                        self.inspection.ready = false;
                    }
                }
                if previous.running && !status.running {
                    self.disassembly_view_addr = None;
                }
                ctx.request_repaint();
            }
            match result {
                Ok(VideoRead::Frame(image)) => next_image = image,
                Ok(_) => {}
                Err(error) => {
                    self.file_error = Some(format!("Could not read machine image: {error}"))
                }
            }
        }
        crate::layout::file_dialogs::limit_dialog_focus(self, ctx);
        let in_session = self.player.screen == crate::player::PlayerScreen::Session;
        if in_session {
            render_top_menu_bar(self, ui);
            if self.view_mode == ViewMode::ScreenOnly {
                // Capture the displayed frame before installing the pending image.
                self.render_player_controls(ui);
            }
        }
        if first_pass {
            // The session controls can open Saved progress in the same pass
            // as keys queued by the window. Dispatch the keys after that click.
            self.handle_keyboard_input(ctx);
        }
        self.render_drive_disk_choice(ctx);
        if let Some((info, image)) = next_image {
            self.host.presented = Some(info);
            self.screen_texture.update(ctx, image);
        }
        if !in_session {
            self.render_player_screen(ui);
            self.finish_ui_pass(ctx, first_pass);
            return;
        }
        self.inspection.frozen = (self.host.continuous_requested()
            || (self.host.inspector_revision.is_none() && self.host.data.is_some()))
            && (self.inspection.ready || self.host.status.pause_pending);
        let screen_texture = self.screen_texture.image();

        // --- RENDER BASED ON ACTIVE VIEW MODE ---
        if self.view_mode == ViewMode::ScreenOnly {
            // Game view never invokes inspection panels or their data queries.
            egui::CentralPanel::default().show(ui, |ui| {
                render_amiga_screen(ui, screen_texture, self.screen_scale, false);
            });
        } else {
            // Mode B: Full Developer Studio GUI (Responsive Multi-Tier Architecture)
            let Some(mut data) = self.host.data.take() else {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("Preparing debugger inspection...");
                });
                self.finish_ui_pass(ctx, first_pass);
                return;
            };
            let original_breakpoints = data.debugger.breakpoints.clone();
            if !self.inspection.frozen {
                self.inspection.refresh_status(&data);
                self.inspection.engine.refresh(&data);
                if self.show_microcode {
                    self.inspection
                        .microcode
                        .refresh(&data.cpu, data.memory.chip_ram_blocked);
                }
            }
            let tokens = self.theme.tokens();

            let screen_rect = ctx.viewport_rect();
            let total_w = screen_rect.width();
            let layout_tier = LayoutTier::from_width(total_w);

            let left_default = match layout_tier {
                LayoutTier::FullHdWide => 360.0,
                LayoutTier::StandardDesktop => 340.0,
                LayoutTier::Compact => 290.0,
            };
            let left_min = if layout_tier == LayoutTier::Compact {
                260.0
            } else {
                280.0
            };
            let right_default = match layout_tier {
                LayoutTier::FullHdWide => 540.0,
                LayoutTier::StandardDesktop => 480.0,
                LayoutTier::Compact => 350.0,
            };
            let right_min = if layout_tier == LayoutTier::Compact {
                330.0
            } else {
                360.0
            };

            // Column 1: Left Dock (Execution, CPU Registers, Engine Status & Microcode)
            egui::Panel::left("left_dock")
                .resizable(true)
                .default_size(left_default)
                .size_range(left_min..=480.0)
                .frame(
                    egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin {
                        left: 8,
                        right: 4,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("left_dock_scroll")
                        .auto_shrink([false, false])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                        )
                        .show(ui, |ui| {
                            if self.host.status.running {
                                ui.disable();
                            }
                            render_registers(
                                ui,
                                &mut self.inspection.registers,
                                !self.inspection.frozen,
                                !self.host.status.running && self.host.can_edit(),
                                &tokens,
                                &data.cpu,
                                &mut self.host.commands,
                                data.previous_cpu.as_ref(),
                                &mut self.active_reg_edit,
                            );
                            ui.add_space(3.0);
                            render_engine_status(ui, &tokens, &self.inspection.engine);
                            if self.show_microcode {
                                ui.add_space(3.0);
                                render_microcode(ui, &self.inspection.microcode);
                            }
                        });
                });

            // Column 3: Right Dock (Memory Hex + Tools)
            egui::Panel::right("right_dock")
                .resizable(true)
                .default_size(right_default)
                .size_range(right_min..=680.0)
                .frame(
                    egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin {
                        left: 4,
                        right: 8,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ui, |ui| {
                    if self.host.status.running {
                        ui.disable();
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        // 1. Bottom Zone: Tool Panels docked to bottom (rendered first in bottom_up)
                        let bottom_h = self.right_dock_bottom_height.clamp(50.0, 420.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), bottom_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("right_dock_tools_scroll")
                                    .auto_shrink([false, false])
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                    )
                                    .show(ui, |ui| {
                                        let tools_resp = ui.vertical(|ui| {
                                            render_memory_search(
                                                ui,
                                                &data.memory,
                                                &mut self.hex_base_addr,
                                                &mut self.memory_search_state,
                                            );
                                            ui.add_space(6.0);
                                            render_breakpoints_panel(
                                                &mut data.debugger.breakpoints,
                                                &mut self.inspection.breakpoints,
                                                !self.inspection.frozen,
                                                &mut self.breakpoint_form,
                                                ui,
                                            );
                                            if layout_tier != LayoutTier::FullHdWide {
                                                ui.add_space(6.0);
                                                render_trace_log(self, ui, &data);
                                            }
                                        });
                                        let measured_h = tools_resp.response.rect.height();
                                        if measured_h > 30.0 {
                                            self.right_dock_bottom_height = measured_h + 8.0;
                                            let tools_h_id =
                                                egui::Id::unique("right_dock_tools_measured_h");
                                            ui.data_mut(|d| {
                                                d.insert_temp(
                                                    tools_h_id,
                                                    self.right_dock_bottom_height,
                                                )
                                            });
                                        }
                                    });
                            },
                        );

                        ui.add_space(4.0);

                        // 2. Top Zone: Memory Hex Editor (takes all remaining vertical space)
                        let remaining_h = ui.available_height();
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), remaining_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                render_memory_hex(
                                    ui,
                                    &mut self.inspection.memory,
                                    !self.inspection.frozen,
                                    !self.host.status.running && self.host.can_edit(),
                                    &data.memory,
                                    &mut self.host.commands,
                                    &mut data.debugger.breakpoints,
                                    &mut self.hex_base_addr,
                                    &mut self.memory_selected_addr,
                                    &mut self.hex_edit_buffer,
                                    if !self.inspection.frozen && data.previous_cpu.is_some() {
                                        Some((&data.previous_bytes, data.previous_base))
                                    } else {
                                        None
                                    },
                                    &tokens,
                                );
                            },
                        );
                    });
                });

            // Column 2: Central Viewport
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin {
                        left: 4,
                        right: 4,
                        top: 6,
                        bottom: 6,
                    }),
                )
                .show(ui, |ui| {
                    match layout_tier {
                        LayoutTier::FullHdWide => {
                            // 4-Pane Studio Workbench Layout (Split Central Area)
                            let total_w = ui.available_width();
                            let total_h = ui.available_height();
                            let min_disasm_w = 280.0_f32;
                            let max_disasm_w = (total_w - 380.0).max(min_disasm_w);
                            let disasm_w = self.disasm_pane_width.clamp(min_disasm_w, max_disasm_w);

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                // Sub-column 0: Dedicated Full-height Disassembly Stream
                                ui.allocate_ui_with_layout(
                                    egui::vec2(disasm_w, total_h),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        render_disassembly(
                                            ui,
                                            &mut self.inspection.disassembly,
                                            !self.inspection.frozen,
                                            !self.host.status.running && self.host.can_edit(),
                                            &data.cpu,
                                            &data.memory,
                                            &mut data.debugger,
                                            &data.temporal,
                                            &mut self.host.commands,
                                            &mut self.goto_addr_str,
                                            &mut self.disassembly_view_addr,
                                            &mut self.disassembly_selected_addr,
                                            &mut self.active_disasm_edit,
                                            &tokens,
                                        );
                                    },
                                );

                                // Draggable Vertical Splitter between Disassembly and CRT/Trace
                                let splitter_resp = ui.allocate_response(
                                    egui::vec2(6.0, total_h),
                                    egui::Sense::click_and_drag(),
                                );
                                let splitter_hovered =
                                    splitter_resp.hovered() || splitter_resp.dragged();
                                let splitter_color = if splitter_hovered {
                                    tokens.border_active
                                } else {
                                    tokens.border_subtle
                                };
                                ui.painter().vline(
                                    splitter_resp.rect.center().x,
                                    ui.max_rect().y_range(),
                                    egui::Stroke::new(
                                        if splitter_hovered { 2.0_f32 } else { 1.0_f32 },
                                        splitter_color,
                                    ),
                                );
                                if splitter_resp.hovered() || splitter_resp.dragged() {
                                    ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                }
                                if splitter_resp.dragged() {
                                    self.disasm_pane_width = (self.disasm_pane_width
                                        + splitter_resp.drag_delta().x)
                                        .clamp(min_disasm_w, max_disasm_w);
                                }

                                // Sub-column 1: Amiga Screen + Temporal Bar (top) + Trace Log (bottom)
                                let remaining_w = ui.available_width();
                                ui.allocate_ui_with_layout(
                                    egui::vec2(remaining_w, total_h),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        let total_col_h = ui.available_height();
                                        let top_h = self
                                            .crt_pane_height
                                            .clamp(200.0, (total_col_h - 100.0).max(220.0));

                                        ui.allocate_ui_with_layout(
                                            egui::vec2(ui.available_width(), top_h),
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                let avail_w = ui.available_width();
                                                let avail_h =
                                                    (ui.available_height() - 48.0).max(120.0);
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(avail_w, avail_h),
                                                    egui::Layout::top_down(egui::Align::Center),
                                                    |ui| {
                                                        render_amiga_screen(
                                                            ui,
                                                            screen_texture,
                                                            self.screen_scale,
                                                            false,
                                                        );
                                                    },
                                                );

                                                ui.separator();
                                                render_temporal_bar(self, ui, &data.temporal);
                                            },
                                        );

                                        // Draggable Splitter between CRT Screen/Temporal Bar and Trace Log
                                        let crt_splitter = ui.allocate_response(
                                            egui::vec2(ui.available_width(), 6.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        let crt_hovered =
                                            crt_splitter.hovered() || crt_splitter.dragged();
                                        let crt_split_color = if crt_hovered {
                                            tokens.border_active
                                        } else {
                                            tokens.border_subtle
                                        };
                                        ui.painter().hline(
                                            crt_splitter.rect.x_range(),
                                            crt_splitter.rect.center().y,
                                            egui::Stroke::new(
                                                if crt_hovered { 2.0_f32 } else { 1.0_f32 },
                                                crt_split_color,
                                            ),
                                        );
                                        if crt_splitter.hovered() || crt_splitter.dragged() {
                                            ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
                                        }
                                        if crt_splitter.dragged() {
                                            self.crt_pane_height = (self.crt_pane_height
                                                + crt_splitter.drag_delta().y)
                                                .clamp(200.0, (total_col_h - 100.0).max(220.0));
                                        }

                                        ui.add_space(2.0);
                                        render_trace_log(self, ui, &data);
                                    },
                                );
                            });
                        }
                        LayoutTier::StandardDesktop | LayoutTier::Compact => {
                            // Stacked Adaptive 3-Column Layout
                            let total_h = ui.available_height();
                            let screen_h = (total_h * 0.38).clamp(130.0, 260.0);

                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), screen_h),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    render_amiga_screen(
                                        ui,
                                        screen_texture,
                                        self.screen_scale,
                                        false,
                                    );
                                },
                            );

                            ui.separator();
                            render_temporal_bar(self, ui, &data.temporal);
                            ui.separator();

                            render_disassembly(
                                ui,
                                &mut self.inspection.disassembly,
                                !self.inspection.frozen,
                                !self.host.status.running && self.host.can_edit(),
                                &data.cpu,
                                &data.memory,
                                &mut data.debugger,
                                &data.temporal,
                                &mut self.host.commands,
                                &mut self.goto_addr_str,
                                &mut self.disassembly_view_addr,
                                &mut self.disassembly_selected_addr,
                                &mut self.active_disasm_edit,
                                &tokens,
                            );
                        }
                    }
                });
            if data.debugger.breakpoints != original_breakpoints {
                self.host.queue(HostCommand::SetBreakpoints(
                    data.debugger.breakpoints.clone(),
                ));
            }
            if !self.host.status.running
                && self.host.can_edit()
                && data.previous_base != self.hex_base_addr
            {
                self.host
                    .queue(HostCommand::SetMemoryBase(self.hex_base_addr));
            }
            self.host.data = Some(data);
        }

        if self.view_mode == ViewMode::Developer && !self.inspection.frozen {
            self.inspection.ready = true;
        }
        self.finish_ui_pass(ctx, first_pass);
    }
}
