// src/main.rs

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let weak = app.as_weak();

    app.on_submit(move || {
        if let Some(app) = weak.upgrade() {
            let name = app.get_customer_name();
            let message = if name.trim().is_empty() {
                "نام مشتری را وارد کنید".into()
            } else {
                format!("«{name}» با موفقیت ثبت آزمایشی شد").into()
            };
            app.set_status_message(message);
        }
    });

    app.run()
}
