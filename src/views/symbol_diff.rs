use egui::{
    text::LayoutJob, CollapsingHeader, Color32, Rgba, ScrollArea, SelectableLabel, Ui, Widget,
};
use egui_extras::{Size, StripBuilder};

use crate::{
    app::{SymbolReference, View, ViewConfig, ViewState},
    jobs::objdiff::BuildStatus,
    obj::{ObjInfo, ObjSection, ObjSectionKind, ObjSymbol},
    views::write_text,
};

pub fn match_color_for_symbol(match_percent: f32) -> Color32 {
    if match_percent == 100.0 {
        Color32::GREEN
    } else if match_percent >= 50.0 {
        Color32::LIGHT_BLUE
    } else {
        Color32::RED
    }
}

fn symbol_context_menu_ui(ui: &mut Ui, symbol: &ObjSymbol) {
    ui.scope(|ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
        ui.style_mut().wrap = Some(false);

        if let Some(name) = &symbol.demangled_name {
            if ui.button(format!("Copy \"{name}\"")).clicked() {
                ui.output().copied_text = name.clone();
                ui.close_menu();
            }
        }
        if ui.button(format!("Copy \"{}\"", symbol.name)).clicked() {
            ui.output().copied_text = symbol.name.clone();
            ui.close_menu();
        }
    });
}

fn symbol_hover_ui(ui: &mut Ui, symbol: &ObjSymbol) {
    ui.scope(|ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
        ui.style_mut().wrap = Some(false);

        ui.colored_label(Color32::WHITE, format!("Name: {}", symbol.name));
        ui.colored_label(Color32::WHITE, format!("Address: {:x}", symbol.address));
        if symbol.size_known {
            ui.colored_label(Color32::WHITE, format!("Size: {:x}", symbol.size));
        } else {
            ui.colored_label(Color32::WHITE, format!("Size: {:x} (assumed)", symbol.size));
        }
    });
}

fn symbol_ui(
    ui: &mut Ui,
    symbol: &ObjSymbol,
    section: Option<&ObjSection>,
    highlighted_symbol: &mut Option<String>,
    selected_symbol: &mut Option<SymbolReference>,
    current_view: &mut View,
    config: &ViewConfig,
) {
    let mut job = LayoutJob::default();
    let name: &str =
        if let Some(demangled) = &symbol.demangled_name { demangled } else { &symbol.name };
    let mut selected = false;
    if let Some(sym) = highlighted_symbol {
        selected = sym == &symbol.name;
    }
    write_text("[", Color32::GRAY, &mut job, config.code_font.clone());
    if symbol.common {
        write_text("c", Color32::from_rgb(0, 255, 255), &mut job, config.code_font.clone());
    } else if symbol.global {
        write_text("g", Color32::GREEN, &mut job, config.code_font.clone());
    } else if symbol.local {
        write_text("l", Color32::GRAY, &mut job, config.code_font.clone());
    }
    if symbol.weak {
        write_text("w", Color32::GRAY, &mut job, config.code_font.clone());
    }
    write_text("] ", Color32::GRAY, &mut job, config.code_font.clone());
    if let Some(match_percent) = symbol.match_percent {
        write_text("(", Color32::GRAY, &mut job, config.code_font.clone());
        write_text(
            &format!("{match_percent:.0}%"),
            match_color_for_symbol(match_percent),
            &mut job,
            config.code_font.clone(),
        );
        write_text(") ", Color32::GRAY, &mut job, config.code_font.clone());
    }
    write_text(name, Color32::WHITE, &mut job, config.code_font.clone());
    let response = SelectableLabel::new(selected, job)
        .ui(ui)
        .context_menu(|ui| symbol_context_menu_ui(ui, symbol))
        .on_hover_ui_at_pointer(|ui| symbol_hover_ui(ui, symbol));
    if response.clicked() {
        if let Some(section) = section {
            if section.kind == ObjSectionKind::Code {
                *selected_symbol = Some(SymbolReference {
                    symbol_name: symbol.name.clone(),
                    section_name: section.name.clone(),
                });
                *current_view = View::FunctionDiff;
            } else if section.kind == ObjSectionKind::Data {
                *selected_symbol = Some(SymbolReference {
                    symbol_name: section.name.clone(),
                    section_name: section.name.clone(),
                });
                *current_view = View::DataDiff;
            }
        }
    } else if response.hovered() {
        *highlighted_symbol = Some(symbol.name.clone());
    }
}

fn symbol_matches_search(symbol: &ObjSymbol, search_str: &str) -> bool {
    search_str.is_empty()
        || symbol.name.contains(search_str)
        || symbol
            .demangled_name
            .as_ref()
            .map(|s| s.to_ascii_lowercase().contains(search_str))
            .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn symbol_list_ui(
    ui: &mut Ui,
    obj: &ObjInfo,
    highlighted_symbol: &mut Option<String>,
    selected_symbol: &mut Option<SymbolReference>,
    current_view: &mut View,
    reverse_function_order: bool,
    search: &mut String,
    config: &ViewConfig,
) {
    ui.text_edit_singleline(search);
    let lower_search = search.to_ascii_lowercase();

    ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap = Some(false);

            if !obj.common.is_empty() {
                CollapsingHeader::new(".comm").default_open(true).show(ui, |ui| {
                    for symbol in &obj.common {
                        symbol_ui(
                            ui,
                            symbol,
                            None,
                            highlighted_symbol,
                            selected_symbol,
                            current_view,
                            config,
                        );
                    }
                });
            }

            for section in &obj.sections {
                CollapsingHeader::new(format!("{} ({:x})", section.name, section.size))
                    .default_open(true)
                    .show(ui, |ui| {
                        if section.kind == ObjSectionKind::Code && reverse_function_order {
                            for symbol in section.symbols.iter().rev() {
                                if !symbol_matches_search(symbol, &lower_search) {
                                    continue;
                                }
                                symbol_ui(
                                    ui,
                                    symbol,
                                    Some(section),
                                    highlighted_symbol,
                                    selected_symbol,
                                    current_view,
                                    config,
                                );
                            }
                        } else {
                            for symbol in &section.symbols {
                                if !symbol_matches_search(symbol, &lower_search) {
                                    continue;
                                }
                                symbol_ui(
                                    ui,
                                    symbol,
                                    Some(section),
                                    highlighted_symbol,
                                    selected_symbol,
                                    current_view,
                                    config,
                                );
                            }
                        }
                    });
            }
        });
    });
}

fn build_log_ui(ui: &mut Ui, status: &BuildStatus) {
    ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap = Some(false);

            ui.colored_label(Color32::from_rgb(255, 0, 0), &status.log);
        });
    });
}

pub fn symbol_diff_ui(ui: &mut Ui, view_state: &mut ViewState) {
    if let (Some(result), highlighted_symbol, selected_symbol, current_view, search) = (
        &view_state.build,
        &mut view_state.highlighted_symbol,
        &mut view_state.selected_symbol,
        &mut view_state.current_view,
        &mut view_state.search,
    ) {
        StripBuilder::new(ui).size(Size::exact(40.0)).size(Size::remainder()).vertical(
            |mut strip| {
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.scope(|ui| {
                                ui.style_mut().override_text_style =
                                    Some(egui::TextStyle::Monospace);
                                ui.style_mut().wrap = Some(false);

                                ui.label("Build target:");
                                if result.first_status.success {
                                    ui.label("OK");
                                } else {
                                    ui.colored_label(Rgba::from_rgb(1.0, 0.0, 0.0), "Fail");
                                }
                            });
                            ui.separator();
                        });
                        strip.cell(|ui| {
                            ui.scope(|ui| {
                                ui.style_mut().override_text_style =
                                    Some(egui::TextStyle::Monospace);
                                ui.style_mut().wrap = Some(false);

                                ui.label("Build base:");
                                if result.second_status.success {
                                    ui.label("OK");
                                } else {
                                    ui.colored_label(Rgba::from_rgb(1.0, 0.0, 0.0), "Fail");
                                }
                            });
                            ui.separator();
                        });
                    });
                });
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                        strip.cell(|ui| {
                            if result.first_status.success {
                                if let Some(obj) = &result.first_obj {
                                    ui.push_id("left", |ui| {
                                        symbol_list_ui(
                                            ui,
                                            obj,
                                            highlighted_symbol,
                                            selected_symbol,
                                            current_view,
                                            view_state.reverse_fn_order,
                                            search,
                                            &view_state.view_config,
                                        );
                                    });
                                }
                            } else {
                                build_log_ui(ui, &result.first_status);
                            }
                        });
                        strip.cell(|ui| {
                            if result.second_status.success {
                                if let Some(obj) = &result.second_obj {
                                    ui.push_id("right", |ui| {
                                        symbol_list_ui(
                                            ui,
                                            obj,
                                            highlighted_symbol,
                                            selected_symbol,
                                            current_view,
                                            view_state.reverse_fn_order,
                                            search,
                                            &view_state.view_config,
                                        );
                                    });
                                }
                            } else {
                                build_log_ui(ui, &result.second_status);
                            }
                        });
                    });
                });
            },
        );
    }
}
