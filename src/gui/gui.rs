use eframe::egui::{ScrollArea, CentralPanel};
use crate::gui::run_tests;

/// GUI with a button to run tests and display results
pub struct GUI {
    pub logs: Vec<String>,
}

impl Default for GUI {
    fn default() -> Self {
        Self { logs: vec![] }
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("oboromi");

            if ui.button("Run Tests").clicked() {
                self.logs = run_tests();
            }

            ui.separator();
            ui.label("Test Results:");
            ScrollArea::vertical().show(ui, |ui| {
                for line in &self.logs {
                    ui.label(line);
                }
            });
        });
    }
}
