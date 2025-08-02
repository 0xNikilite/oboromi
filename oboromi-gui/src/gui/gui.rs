use eframe::egui::{ScrollArea, CentralPanel};
use oboromi_core::tests::run::run_tests;

/// GUI with button to run tests and display results
pub struct GUI {
    pub logs: Vec<String>,
    pub test_thread: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Default for GUI {
    fn default() -> Self {
        Self { 
            logs: vec![],
            test_thread: None,
        }
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("oboromi");
            
            if let Some(handle) = self.test_thread.take() {
                self.logs = handle.join().unwrap();
            }
            
            // Run tests button (disabled while tests are running)
            if self.test_thread.is_none() {
                if ui.button("🧪 Run Dynarmic Tests").clicked() {
                    let ctx = ctx.clone();
                    self.test_thread = Some(std::thread::spawn(move || {
                        ctx.request_repaint(); // Force UI update
                        run_tests()
                    }));
                    self.logs = vec!["Running tests...".to_string()];
                }
            } else {
                ui.label("⏳ Tests running...");
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