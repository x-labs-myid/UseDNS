//! Windows shell policy for the main window and the non-activating tray preview.
use crate::{AppWindow, TrayPreview};
use i_slint_backend_winit::{Backend, BackendBuilder};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    ComponentHandle,
    winit_030::{EventResult, WinitWindowAccessor, winit},
};
use std::{cell::Cell, rc::Rc};
use winit::platform::windows::WindowAttributesExtWindows;

const GWL_EXSTYLE: i32 = -20;
const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
const WS_EX_APPWINDOW: isize = 0x0004_0000;
const WS_EX_NOACTIVATE: isize = 0x0800_0000;

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowLongPtrW(hwnd: *mut std::ffi::c_void, index: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: *mut std::ffi::c_void, index: i32, value: isize) -> isize;
}

fn create_with_backend(
    builder: BackendBuilder,
) -> Result<(AppWindow, TrayPreview), slint::PlatformError> {
    // The hook runs when Slint creates an adapter, before the native HWND exists.
    // Explicitly materialize each adapter while its creation role is selected.
    // The preview's skip_taskbar flag is then kept in winit's own state, including
    // its TaskbarCreated handling, rather than only patching native style bits.
    let creating_preview = Rc::new(Cell::new(false));
    let hook_role = creating_preview.clone();
    let backend = builder
        .with_window_attributes_hook(move |attributes| {
            if hook_role.get() {
                attributes.with_skip_taskbar(true).with_active(false)
            } else {
                attributes
            }
        })
        .build()?;
    slint::platform::set_platform(Box::new(backend))
        .map_err(slint::PlatformError::SetPlatformError)?;

    let main = AppWindow::new()?;
    let _ = main.window();
    creating_preview.set(true);
    let preview = TrayPreview::new()?;
    let _ = preview.window();
    creating_preview.set(false);

    preview.window().on_winit_window_event(|window, _| {
        enforce_preview_style(window);
        EventResult::Propagate
    });
    Ok((main, preview))
}

pub fn create_windows() -> Result<(AppWindow, TrayPreview), slint::PlatformError> {
    create_with_backend(Backend::builder())
}

fn enforce_preview_style(window: &slint::Window) {
    window.with_winit_window(|native| {
        if let Ok(handle) = native.window_handle()
            && let RawWindowHandle::Win32(win32) = handle.as_raw()
        {
            let hwnd = win32.hwnd.get() as *mut std::ffi::c_void;
            // SAFETY: native owns this valid HWND. Preserve all unrelated styles.
            unsafe {
                let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let desired = (current | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) & !WS_EX_APPWINDOW;
                if current != desired {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
                }
            }
        }
    });
}

pub fn show_preview(preview: &TrayPreview) -> Result<(), slint::PlatformError> {
    // Existing windows must be configured before re-showing to avoid activation.
    // On the first show, winit's creation attributes already exclude the taskbar
    // and suppress activation; now that the HWND exists, finish the tool style.
    enforce_preview_style(preview.window());
    preview.show()?;
    enforce_preview_style(preview.window());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::platform::windows::EventLoopBuilderExtWindows;

    fn extended_style(window: &slint::Window) -> isize {
        window
            .with_winit_window(|native| {
                let handle = native.window_handle().unwrap();
                let RawWindowHandle::Win32(win32) = handle.as_raw() else {
                    panic!("expected Windows HWND");
                };
                unsafe { GetWindowLongPtrW(win32.hwnd.get() as _, GWL_EXSTYLE) }
            })
            .expect("native window must exist")
    }

    fn snapshot(window: &slint::Window, name: &str) {
        let pixels = window.take_snapshot().unwrap();
        std::fs::create_dir_all("target/ui-review").unwrap();
        image::save_buffer(
            format!("target/ui-review/{name}.png"),
            pixels.as_bytes(),
            pixels.width(),
            pixels.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }

    fn click(window: &slint::Window, x: f32, y: f32) {
        let position = slint::LogicalPosition::new(x, y);
        window.dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
        window.dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    }

    #[test]
    #[ignore = "renders UI screenshots on an isolated Windows desktop"]
    fn adaptive_metrics_and_documentation_render() {
        let mut event_loop = winit::event_loop::EventLoop::with_user_event();
        event_loop.with_any_thread(true);
        let (main, preview) = create_with_backend(
            Backend::builder()
                .with_event_loop_builder(event_loop)
                .with_renderer_name("software"),
        )
        .unwrap();
        let main_weak = main.as_weak();
        let preview_weak = preview.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
            let main = main_weak.unwrap();
            let preview = preview_weak.unwrap();
            main.set_theme_mode("light".into());
            main.set_language("id".into());
            main.set_download_speed(crate::metrics::format_speed(10.792).into());
            main.set_upload_speed(crate::metrics::format_speed(0.225).into());
            let total = crate::metrics::speed(11.017);
            main.set_total_speed(total.value.into());
            main.set_total_speed_unit(total.unit.into());
            main.set_connection_state(crate::metrics::response_state(1222).into());
            main.set_latency("1222 ms".into());
            main.show().unwrap();
            snapshot(main.window(), "home-id");
            main.set_nav_menu_open(true);
            snapshot(main.window(), "navigation-id");
            main.invoke_navigate(5);
            snapshot(main.window(), "faq-id");
            // Exercise a real pointer click on the first FAQ accordion.
            click(main.window(), 300.0, 215.0);
            snapshot(main.window(), "faq-expanded-id");
            click(main.window(), 470.0, 165.0);
            snapshot(main.window(), "faq-monitoring-id");
            click(main.window(), 300.0, 355.0);
            snapshot(main.window(), "faq-rating-id");
            main.set_language("en".into());
            snapshot(main.window(), "faq-en");
            main.invoke_navigate(4);
            snapshot(main.window(), "settings-en");
            main.window()
                .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(300.0, 600.0),
                    delta_x: 0.0,
                    delta_y: -600.0,
                });
            snapshot(main.window(), "settings-units-en");
            preview.set_language("id".into());
            preview.set_download_speed(main.get_download_speed());
            preview.set_upload_speed(main.get_upload_speed());
            preview.set_connection_state(main.get_connection_state());
            preview.set_latency(main.get_latency());
            show_preview(&preview).unwrap();
            snapshot(preview.window(), "tray-id");
            preview.set_connection_state("Excellent".into());
            snapshot(preview.window(), "tray-five-stars");
            preview.set_connection_state("Unavailable".into());
            snapshot(preview.window(), "tray-unmeasured");
            main.hide().unwrap();
            preview.hide().unwrap();
            slint::quit_event_loop().unwrap();
        });
        slint::run_event_loop_until_quit().unwrap();
    }

    #[test]
    #[ignore = "run separately on an isolated Windows desktop with the native test runner"]
    fn navigation_closes_overlays_for_every_page() {
        let mut event_loop = winit::event_loop::EventLoop::with_user_event();
        event_loop.with_any_thread(true);
        let (main, _preview) = create_with_backend(
            Backend::builder()
                .with_event_loop_builder(event_loop)
                .with_renderer_name("software"),
        )
        .unwrap();
        for source in 0..6 {
            for destination in 0..6 {
                main.invoke_navigate(source);
                main.set_profile_popup_open(true);
                main.set_close_dialog_open(true);
                main.set_nav_menu_open(true);
                main.invoke_navigate(destination);
                assert_eq!(main.get_page(), destination);
                assert!(!main.get_profile_popup_open());
                assert!(!main.get_close_dialog_open());
                assert!(!main.get_nav_menu_open());
                main.invoke_navigate(source);
                assert!(
                    !main.get_profile_popup_open(),
                    "returning must not reopen modal"
                );
            }
        }
        // Booster review navigates first, then deliberately opens a new modal.
        main.invoke_navigate(4);
        main.invoke_navigate(1);
        main.set_profile_popup_open(true);
        assert!(main.get_profile_popup_open());
        main.invoke_navigate(0);
        main.invoke_navigate(1);
        assert!(!main.get_profile_popup_open());
    }

    #[test]
    #[ignore = "run on an isolated Windows desktop to avoid altering the user's taskbar"]
    fn preview_stays_out_of_taskbar_across_show_hide_cycles() {
        let mut event_loop = winit::event_loop::EventLoop::with_user_event();
        event_loop.with_any_thread(true);
        let (main, preview) = create_with_backend(
            Backend::builder()
                .with_event_loop_builder(event_loop)
                .with_renderer_name("software"),
        )
        .unwrap();
        // This test must not run on the interactive desktop: it exercises real
        // native visibility, rather than just checking bitwise helper output.
        main.window()
            .set_position(slint::PhysicalPosition::new(-10000, -10000));
        preview
            .window()
            .set_position(slint::PhysicalPosition::new(-10000, -10000));
        let main_weak = main.as_weak();
        let preview_weak = preview.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(100), move || {
            let main = main_weak.unwrap();
            let preview = preview_weak.unwrap();
            main.show().unwrap();
            assert_ne!(extended_style(main.window()) & WS_EX_APPWINDOW, 0);
            main.hide().unwrap();
            assert!(!main.window().is_visible());
            for _ in 0..3 {
                show_preview(&preview).unwrap();
                let style = extended_style(preview.window());
                assert_ne!(style & WS_EX_TOOLWINDOW, 0);
                assert_ne!(style & WS_EX_NOACTIVATE, 0);
                assert_eq!(style & WS_EX_APPWINDOW, 0);
                assert!(!main.window().is_visible());
                preview.hide().unwrap();
            }
            main.show().unwrap();
            assert_ne!(extended_style(main.window()) & WS_EX_APPWINDOW, 0);
            main.hide().unwrap();
            slint::quit_event_loop().unwrap();
        });
        slint::run_event_loop_until_quit().unwrap();
    }
}
