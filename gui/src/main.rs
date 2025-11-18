use oboromi_gui::gui::GUI;

#[allow(dead_code)]
fn decode_arm64_fields(opcode: u32) -> (u8, u8, u8, u8) {
    let sf = ((opcode >> 31) & 1) as u8;
    let opc = ((opcode >> 29) & 0x3) as u8;
    let rn = ((opcode >> 5) & 0x1F) as u8;
    let rd = (opcode & 0x1F) as u8;
    (sf, opc, rn, rd)
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("oboromi.log")?)
        .apply()?;
    Ok(())
}

fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("oboromi"),
        ..Default::default()
    };

    eframe::run_native(
        "oboromi",
        options,
        Box::new(|_cc| Ok(Box::new(GUI::default()))),
    )
    .expect("Failed to run GUI");
}

fn main() {
    // Initialize logging
    setup_logger().expect("Failed to initialize logger");

    #[cfg(feature = "trace")]
    log::info!("-- TRACING ENABLED --");

    {
        run_gui();
    }
}
