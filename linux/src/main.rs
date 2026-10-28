use reset_explode_linux::benchmark::{self, Results};
use reset_explode_linux::render_bench::{self, RenderPreview, RenderResult};
use reset_explode_linux::{page_text, report_html, snapshot, ReportTheme};
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::os::unix::ffi::OsStrExt;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[link(name = "gtk-4")]
extern "C" {
    fn gtk_init();
    fn gtk_window_present(window: *mut c_void);
    fn gtk_widget_realize(widget: *mut c_void);
    fn gtk_native_get_surface(native: *mut c_void) -> *mut c_void;
    fn gdk_wayland_toplevel_get_type() -> usize;
    fn gdk_wayland_toplevel_set_application_id(surface: *mut c_void, id: *const c_char);
    fn gtk_window_minimize(window: *mut c_void);
    fn gtk_window_maximize(window: *mut c_void);
    fn gtk_window_unmaximize(window: *mut c_void);
    fn gtk_window_is_maximized(window: *mut c_void) -> c_int;
    fn gtk_window_destroy(window: *mut c_void);
    fn gtk_window_set_transient_for(window: *mut c_void, parent: *mut c_void);
    fn gtk_picture_new() -> *mut c_void;
    fn gdk_texture_new_from_bytes(bytes: *mut c_void, error: *mut *mut c_void) -> *mut c_void;
    fn gtk_picture_set_paintable(picture: *mut c_void, paintable: *mut c_void);
    fn gtk_picture_set_content_fit(picture: *mut c_void, fit: c_int);
    fn gdk_memory_texture_new(
        width: c_int,
        height: c_int,
        format: c_int,
        bytes: *mut c_void,
        stride: usize,
    ) -> *mut c_void;
    fn gtk_window_set_title(window: *mut c_void, title: *const c_char);
    fn gtk_window_set_default_icon_name(name: *const c_char);
    fn gtk_window_set_default_size(window: *mut c_void, width: c_int, height: c_int);
    fn gtk_scrolled_window_set_child(scroll: *mut c_void, child: *mut c_void);
    fn gtk_scrolled_window_new() -> *mut c_void;
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut c_void;
    fn gtk_box_append(b: *mut c_void, child: *mut c_void);
    fn gtk_box_remove(b: *mut c_void, child: *mut c_void);
    fn gtk_widget_get_first_child(widget: *mut c_void) -> *mut c_void;
    fn gtk_label_new(text: *const c_char) -> *mut c_void;
    fn gtk_size_group_new(mode: c_int) -> *mut c_void;
    fn gtk_size_group_add_widget(group: *mut c_void, widget: *mut c_void);
    fn gtk_label_set_xalign(label: *mut c_void, xalign: f32);
    fn gtk_label_set_text(label: *mut c_void, text: *const c_char);
    fn gtk_label_set_wrap(label: *mut c_void, wrap: c_int);
    fn gtk_widget_set_vexpand(widget: *mut c_void, expand: c_int);
    fn gtk_widget_set_hexpand(widget: *mut c_void, expand: c_int);
    fn gtk_widget_set_size_request(widget: *mut c_void, width: c_int, height: c_int);
    fn gtk_widget_set_margin_start(widget: *mut c_void, margin: c_int);
    fn gtk_widget_set_margin_end(widget: *mut c_void, margin: c_int);
    fn gtk_widget_set_margin_top(widget: *mut c_void, margin: c_int);
    fn gtk_widget_set_margin_bottom(widget: *mut c_void, margin: c_int);
    fn gtk_widget_add_css_class(widget: *mut c_void, class: *const c_char);
    fn gtk_widget_remove_css_class(widget: *mut c_void, class: *const c_char);
    fn gtk_button_new() -> *mut c_void;
    fn gtk_button_new_from_icon_name(name: *const c_char) -> *mut c_void;
    fn gtk_button_set_icon_name(button: *mut c_void, name: *const c_char);
    fn gtk_widget_set_tooltip_text(widget: *mut c_void, text: *const c_char);
    fn gtk_button_set_child(button: *mut c_void, child: *mut c_void);
    fn gtk_widget_set_sensitive(widget: *mut c_void, sensitive: c_int);
    fn gtk_widget_set_visible(widget: *mut c_void, visible: c_int);
    fn gtk_file_chooser_native_new(
        title: *const c_char,
        parent: *mut c_void,
        action: c_int,
        accept: *const c_char,
        cancel: *const c_char,
    ) -> *mut c_void;
    fn gtk_file_chooser_set_current_name(chooser: *mut c_void, name: *const c_char);
    fn gtk_file_chooser_get_file(chooser: *mut c_void) -> *mut c_void;
    fn gtk_native_dialog_show(dialog: *mut c_void);
    fn gtk_native_dialog_destroy(dialog: *mut c_void);
    fn gtk_css_provider_new() -> *mut c_void;
    fn gtk_css_provider_load_from_data(provider: *mut c_void, data: *const c_char, len: isize);
    fn gtk_style_context_add_provider_for_display(
        display: *mut c_void,
        provider: *mut c_void,
        priority: c_uint,
    );
    fn gdk_display_get_default() -> *mut c_void;
}
#[link(name = "adwaita-1")]
extern "C" {
    fn adw_init();
    fn adw_window_new() -> *mut c_void;
    fn adw_window_set_content(window: *mut c_void, content: *mut c_void);
    fn adw_header_bar_new() -> *mut c_void;
    fn adw_header_bar_set_decoration_layout(header: *mut c_void, layout: *const c_char);
    fn adw_header_bar_pack_end(header: *mut c_void, widget: *mut c_void);
    fn adw_style_manager_get_default() -> *mut c_void;
    fn adw_style_manager_set_color_scheme(manager: *mut c_void, scheme: c_int);
}
#[link(name = "gobject-2.0")]
extern "C" {
    fn g_signal_connect_data(
        instance: *mut c_void,
        signal: *const c_char,
        handler: *const c_void,
        data: *mut c_void,
        destroy: *mut c_void,
        flags: c_uint,
    ) -> u64;
    fn g_object_unref(object: *mut c_void);
    fn g_type_check_instance_is_a(instance: *mut c_void, g_type: usize) -> c_int;
}
#[link(name = "gio-2.0")]
extern "C" {
    fn g_file_get_path(file: *mut c_void) -> *mut c_char;
}
#[link(name = "glib-2.0")]
extern "C" {
    fn g_main_loop_new(context: *mut c_void, running: c_int) -> *mut c_void;
    fn g_main_loop_run(loop_: *mut c_void);
    fn g_main_loop_quit(loop_: *mut c_void);
    fn g_main_loop_unref(loop_: *mut c_void);
    fn g_timeout_add_seconds(
        interval: c_uint,
        function: extern "C" fn(*mut c_void) -> c_int,
        data: *mut c_void,
    ) -> c_uint;
    fn g_source_remove(tag: c_uint) -> c_int;
    fn g_free(memory: *mut c_void);
    fn g_set_prgname(name: *const c_char);
    fn g_bytes_new(data: *const c_void, size: usize) -> *mut c_void;
    fn g_bytes_unref(bytes: *mut c_void);
}

// Palette inspired by the Night Owl theme by Sarah Drasner.
const DARK_CSS: &str = r#"
window { background: #011627; color: #d6deeb; }
headerbar { background: #011627; box-shadow: none; border-bottom: 1px solid #1d3b53; }
.reset-sidebar { background: #01111d; border: 1px solid #1d3b53; border-radius: 20px; padding: 18px 12px; }
.reset-brand { color: #d6deeb; font-size: 15px; font-weight: 800; padding: 4px 4px; }
.reset-group { color: #a6accd; font-size: 10px; font-weight: 700; letter-spacing: 1px; padding: 16px 12px 6px; }
button.reset-nav { background: transparent; border: 0; box-shadow: none; color: #c5d2e3; border-radius: 10px; padding: 10px 12px; font-weight: 500; }
button.reset-nav:hover { background: #1d3b53; color: #fff; }
button.reset-nav.active { background: #1d3b53; color: #82aaff; font-weight: 700; }
.reset-title { font-size: 27px; font-weight: 700; color: #d6deeb; }
.reset-subtitle { color: #a6accd; font-size: 13px; }
.reset-card { background: #0b253a; border: 1px solid #1d3b53; border-radius: 16px; padding: 20px; }
.reset-card-title { color: #82aaff; font-size: 11px; font-weight: 700; letter-spacing: 1px; margin-bottom: 10px; }
.reset-row { border-bottom: 1px solid #1d3b53; padding: 9px 0; }
.reset-key { color: #a6accd; font-size: 13px; }
.reset-value { color: #d6deeb; font-size: 13px; }
button.reset-run { background: #82aaff; color: #011627; border-radius: 10px; padding: 12px 20px; font-weight: 700; }
button.reset-run .reset-value { color: #011627; }
button.reset-run:disabled { opacity: 0.5; }
.reset-footer { color: #a6accd; font-size: 11px; padding: 16px 4px; }
"#;
const LIGHT_CSS: &str = r#"
window { background: #f6f6f6; color: #403f53; }
headerbar { background: #f6f6f6; box-shadow: none; border-bottom: 1px solid #d8dfe3; }
.reset-sidebar { background: #fff; border: 1px solid #d8dfe3; border-radius: 20px; padding: 18px 12px; }
.reset-brand { color: #403f53; font-size: 15px; font-weight: 800; padding: 4px 4px; }
.reset-group { color: #626b7d; font-size: 10px; font-weight: 700; letter-spacing: 1px; padding: 16px 12px 6px; }
button.reset-nav { background: transparent; border: 0; box-shadow: none; color: #495365; border-radius: 10px; padding: 10px 12px; font-weight: 500; }
button.reset-nav:hover { background: #e8edf3; color: #263247; }
button.reset-nav.active { background: #e0e9f5; color: #2458a4; font-weight: 700; }
.reset-title { font-size: 27px; font-weight: 700; color: #403f53; }
.reset-subtitle { color: #626b7d; font-size: 13px; }
.reset-card { background: #fff; border: 1px solid #d8dfe3; border-radius: 16px; padding: 20px; }
.reset-card-title { color: #365f9d; font-size: 11px; font-weight: 700; letter-spacing: 1px; margin-bottom: 10px; }
.reset-row { border-bottom: 1px solid #e4e9ed; padding: 9px 0; }
.reset-key { color: #626b7d; font-size: 13px; }
.reset-value { color: #403f53; font-size: 13px; }
button.reset-run { background: #315b9a; color: #fff; border-radius: 10px; padding: 12px 20px; font-weight: 700; }
button.reset-run .reset-value { color: #fff; }
button.reset-run:disabled { opacity: 0.5; }
.reset-footer { color: #626b7d; font-size: 11px; padding: 16px 4px; }
"#;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Dark,
    Light,
}
fn theme_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|x| !x.is_empty())
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|x| std::path::PathBuf::from(x).join(".config"))
        })?;
    Some(base.join("reset-explode").join("theme"))
}
fn load_theme() -> Theme {
    if theme_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .as_deref()
        .map(str::trim)
        == Some("light")
    {
        Theme::Light
    } else {
        Theme::Dark
    }
}
fn save_theme(theme: Theme) -> std::io::Result<()> {
    if let Some(path) = theme_path() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            if theme == Theme::Dark {
                "dark\n"
            } else {
                "light\n"
            },
        )?;
    }
    Ok(())
}
fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "�")).expect("texto sem NUL")
}
unsafe fn class(widget: *mut c_void, name: &str) {
    gtk_widget_add_css_class(widget, cstring(name).as_ptr());
}
unsafe fn label(text: &str, style: &str) -> *mut c_void {
    let widget = gtk_label_new(cstring(text).as_ptr());
    gtk_label_set_xalign(widget, 0.0);
    gtk_label_set_wrap(widget, 1);
    class(widget, style);
    widget
}
enum BenchResult {
    Local(Results),
    Render(RenderResult),
}
struct UiState {
    content: *mut c_void,
    page: &'static str,
    buttons: Vec<(*mut c_void, &'static str)>,
    benchmark: Option<String>,
    closing: bool,
    main_loop: *mut c_void,
    running: Option<Receiver<Result<BenchResult, String>>>,
    cancel: Option<Arc<AtomicBool>>,
    progress: Option<Arc<AtomicUsize>>,
    theme: Theme,
    provider: *mut c_void,
    theme_label: *mut c_void,
    window: *mut c_void,
    export_status: *mut c_void,
    preview_window: *mut c_void,
    preview_picture: *mut c_void,
    preview_status: *mut c_void,
    preview_cancel: *mut c_void,
    preview: Option<Arc<RenderPreview>>,
}
struct NavItem {
    state: *mut UiState,
    page: &'static str,
}
fn title(page: &str) -> &'static str {
    match page {
        "Visão geral" => "Visão geral",
        "SISTEMA" => "Sistema",
        "SENSORES" => "Sensores",
        "PLACA DE VÍDEO" => "Placa de vídeo",
        "ARMAZENAMENTO" => "Discos",
        "ENERGIA" => "Energia",
        "REDE" => "Rede",
        "BIBLIOTECA STEAM" => "Biblioteca Steam",
        "PORTAS SERIAIS" => "Portas seriais",
        "BENCHMARK" => "RESET BENCH",
        "SOBRE" => "Sobre",
        _ => "RESET EXPLODE",
    }
}
unsafe fn render(state: &mut UiState) {
    loop {
        let child = gtk_widget_get_first_child(state.content);
        if child.is_null() {
            break;
        }
        gtk_box_remove(state.content, child);
    }
    gtk_box_append(state.content, label(title(state.page), "reset-title"));
    if state.page == "BENCHMARK" {
        gtk_box_append(
            state.content,
            label("Benchmarks de CPU para Linux", "reset-subtitle"),
        );
        let render_card = gtk_box_new(1, 12);
        class(render_card, "reset-card");
        gtk_box_append(
            render_card,
            label("RENDERIZAÇÃO DA CPU · RESET BENCH", "reset-card-title"),
        );
        gtk_box_append(render_card, label("Cena de path tracing do projeto original: 960 × 540 pixels, 1.024 amostras por pixel e uma passada. Pode demorar vários minutos.", "reset-value"));
        for (caption, callback) in [
            (
                "RENDERIZAR · SINGLE-CORE",
                start_render_single as *const c_void,
            ),
            (
                "RENDERIZAR · MULTI-CORE",
                start_render_multi as *const c_void,
            ),
        ] {
            let button = gtk_button_new();
            class(button, "reset-run");
            gtk_button_set_child(button, label(caption, "reset-value"));
            gtk_widget_set_sensitive(button, if state.running.is_some() { 0 } else { 1 });
            g_signal_connect_data(
                button,
                cstring("clicked").as_ptr(),
                callback,
                (state as *mut UiState).cast(),
                std::ptr::null_mut(),
                0,
            );
            gtk_box_append(render_card, button);
        }
        if let Some(progress) = &state.progress {
            let percent = progress.load(Ordering::Relaxed) as f64 * 100.0 / (960.0 * 540.0);
            gtk_box_append(
                render_card,
                label(
                    &format!("Progresso: {:.1}%", percent.min(100.0)),
                    "reset-value",
                ),
            );
            let cancel_button = gtk_button_new();
            gtk_button_set_child(cancel_button, label("CANCELAR RENDERIZAÇÃO", "reset-value"));
            g_signal_connect_data(
                cancel_button,
                cstring("clicked").as_ptr(),
                cancel_render as *const c_void,
                (state as *mut UiState).cast(),
                std::ptr::null_mut(),
                0,
            );
            gtk_box_append(render_card, cancel_button);
        }
        gtk_box_append(state.content, render_card);
        let card = gtk_box_new(1, 12);
        class(card, "reset-card");
        gtk_box_append(
            card,
            label("TESTES RÁPIDOS LOCAIS · CPU", "reset-card-title"),
        );
        gtk_box_append(card, label("O teste usa os processadores disponíveis. Os resultados dependem da carga, temperatura e limites de energia.", "reset-value"));
        let button = gtk_button_new();
        class(button, "reset-run");
        gtk_button_set_child(
            button,
            label(
                if state.running.is_some() {
                    "TESTE EM ANDAMENTO…"
                } else {
                    "INICIAR BENCHMARK"
                },
                "reset-value",
            ),
        );
        gtk_widget_set_sensitive(button, if state.running.is_some() { 0 } else { 1 });
        g_signal_connect_data(
            button,
            cstring("clicked").as_ptr(),
            start_benchmark as *const c_void,
            (state as *mut UiState).cast(),
            std::ptr::null_mut(),
            0,
        );
        gtk_box_append(card, button);
        gtk_box_append(state.content, card);
        let result = gtk_box_new(1, 8);
        class(result, "reset-card");
        gtk_box_append(result, label("RESULTADOS", "reset-card-title"));
        let message = if state.progress.is_some() {
            "Renderização em andamento…"
        } else if state.running.is_some() {
            "Executando CPU single-core e CPU multi-core…"
        } else {
            state
                .benchmark
                .as_deref()
                .unwrap_or("Nenhum teste executado nesta sessão.")
        };
        gtk_box_append(result, label(message, "reset-value"));
        gtk_box_append(state.content, result);
        gtk_box_append(state.content, label("A nota da renderização usa a fórmula do original, mas a equivalência entre Windows e Linux ainda não foi validada. Os testes rápidos são independentes.", "reset-footer"));
        return;
    }
    gtk_box_append(
        state.content,
        label(
            if state.page == "SOBRE" {
                "Informações do projeto"
            } else {
                "Monitoramento do sistema · atualização a cada 3 segundos"
            },
            "reset-subtitle",
        ),
    );
    let text = if state.page == "SOBRE" {
        "SOBRE O PROJETO\nCriador: Jose Jacsan\nLicença: GNU General Public License v3 (GPLv3)"
            .to_owned()
    } else {
        page_text(&snapshot(), state.page)
    };
    for section in text.split("\n\n") {
        let mut lines = section.lines();
        let Some(heading) = lines.next() else {
            continue;
        };
        if heading.starts_with("RESET EXPLODE") || heading.is_empty() {
            continue;
        }
        let card = gtk_box_new(1, 0);
        class(card, "reset-card");
        gtk_box_append(card, label(heading, "reset-card-title"));
        // Keep all keys in this card the same width, even when a label is long.
        let key_widths = gtk_size_group_new(1); // GTK_SIZE_GROUP_HORIZONTAL
        for line in lines {
            let row = gtk_box_new(0, 16);
            class(row, "reset-row");
            if let Some((key, value)) = line.split_once(": ") {
                let k = label(key, "reset-key");
                gtk_widget_set_size_request(k, 185, -1);
                gtk_size_group_add_widget(key_widths, k);
                gtk_box_append(row, k);
                let v = label(value, "reset-value");
                gtk_widget_set_hexpand(v, 1);
                gtk_box_append(row, v);
            } else {
                gtk_box_append(row, label(line, "reset-value"));
            }
            gtk_box_append(card, row);
        }
        g_object_unref(key_widths);
        gtk_box_append(state.content, card);
    }
    gtk_box_append(
        state.content,
        label("RESET EXPLODE · LINUX", "reset-footer"),
    );
}
fn describe_result(result: Result<BenchResult, String>) -> String {
    match result {
        Ok(BenchResult::Local(r)) => format!("CPU single-core     {:.1} milhões de iterações/s\nCPU multi-core      {:.1} milhões de iterações/s  ·  {} threads\n", r.single_mops, r.multi_mops, r.threads),
        Ok(BenchResult::Render(r)) => format!("Renderização de CPU: {:.1} pontos · {} threads\nAmostras: {} · Tempo: {:.1} s\nNota preliminar: equivalência com o original ainda não validada.", r.score, r.threads, r.samples, r.seconds),
        Err(e) => format!("Não foi possível concluir o teste: {e}"),
    }
}
extern "C" fn start_benchmark(_: *mut c_void, data: *mut c_void) {
    unsafe {
        let state = &mut *(data as *mut UiState);
        if state.running.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("reset-benchmark".into())
            .spawn(move || {
                let _ = tx.send(benchmark::run(Duration::from_secs(2)).map(BenchResult::Local));
            }) {
            Ok(_) => {
                state.running = Some(rx);
                state.benchmark = None;
            }
            Err(e) => state.benchmark = Some(format!("Não foi possível iniciar o teste: {e}")),
        }
        render(state);
    }
}
unsafe fn launch_render(data: *mut c_void, single: bool) {
    let state = &mut *(data as *mut UiState);
    if state.running.is_some() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicUsize::new(0));
    let c = cancel.clone();
    let p = progress.clone();
    let preview = Arc::new(RenderPreview::new(960, 540));
    let worker_preview = preview.clone();
    let threads = if single {
        1
    } else {
        std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(1)
    };
    match std::thread::Builder::new()
        .name("reset-render".into())
        .spawn(move || {
            let _ = tx.send(
                render_bench::run_with_preview(threads, c, p, worker_preview)
                    .map(BenchResult::Render),
            );
        }) {
        Ok(_) => {
            state.running = Some(rx);
            state.cancel = Some(cancel);
            state.progress = Some(progress);
            state.benchmark = None;
            open_preview(state, preview);
        }
        Err(e) => state.benchmark = Some(format!("Não foi possível iniciar a renderização: {e}")),
    }
    render(state);
}
extern "C" fn start_render_single(_: *mut c_void, data: *mut c_void) {
    unsafe { launch_render(data, true) }
}
extern "C" fn start_render_multi(_: *mut c_void, data: *mut c_void) {
    unsafe { launch_render(data, false) }
}
extern "C" fn cancel_render(_: *mut c_void, data: *mut c_void) {
    unsafe {
        if let Some(cancel) = &(*(data as *const UiState)).cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}
unsafe fn draw_preview(state: &UiState) {
    if state.preview_picture.is_null() {
        return;
    }
    if let Some(preview) = &state.preview {
        let frame = preview.rgba();
        let bytes = g_bytes_new(frame.as_ptr().cast(), frame.len());
        let texture = gdk_memory_texture_new(960, 540, 5, bytes, 960 * 4); // RGBA, opaque
        gtk_picture_set_paintable(state.preview_picture, texture);
        g_object_unref(texture);
        g_bytes_unref(bytes);
    }
}
extern "C" fn preview_update(data: *mut c_void) -> c_int {
    unsafe {
        let state = &*(data as *const UiState);
        if !state.preview_window.is_null() && state.progress.is_some() {
            draw_preview(state);
            let progress = state.progress.as_ref().unwrap().load(Ordering::Relaxed) as f64 * 100.0
                / (960.0 * 540.0);
            gtk_label_set_text(
                state.preview_status,
                cstring(&format!(
                    "Renderizando · {:.1}% · fechar esta janela cancela o teste",
                    progress.min(100.0)
                ))
                .as_ptr(),
            );
        }
    }
    1
}
extern "C" fn preview_close(_: *mut c_void, data: *mut c_void) -> c_int {
    unsafe {
        let state = &mut *(data as *mut UiState);
        if let Some(cancel) = &state.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        state.preview_window = std::ptr::null_mut();
        state.preview_picture = std::ptr::null_mut();
        state.preview_status = std::ptr::null_mut();
        state.preview_cancel = std::ptr::null_mut();
        state.preview = None;
    }
    0
}
extern "C" fn minimize_preview(_: *mut c_void, window: *mut c_void) {
    unsafe {
        gtk_window_minimize(window);
    }
}
extern "C" fn maximize_preview(_: *mut c_void, window: *mut c_void) {
    unsafe {
        if gtk_window_is_maximized(window) != 0 {
            gtk_window_unmaximize(window);
        } else {
            gtk_window_maximize(window);
        }
    }
}
extern "C" fn update_maximize_button(window: *mut c_void, _: *mut c_void, button: *mut c_void) {
    unsafe {
        let maximized = gtk_window_is_maximized(window) != 0;
        gtk_button_set_icon_name(
            button,
            cstring(if maximized {
                "window-restore-symbolic"
            } else {
                "window-maximize-symbolic"
            })
            .as_ptr(),
        );
        gtk_widget_set_tooltip_text(
            button,
            cstring(if maximized {
                "Restaurar tamanho"
            } else {
                "Maximizar"
            })
            .as_ptr(),
        );
    }
}
unsafe fn open_preview(state: &mut UiState, preview: Arc<RenderPreview>) {
    if !state.preview_window.is_null() {
        gtk_window_destroy(state.preview_window);
    }
    let window = adw_window_new();
    gtk_window_set_title(
        window,
        cstring("RESET BENCH · Renderização de CPU").as_ptr(),
    );
    gtk_window_set_default_size(window, 1000, 680);
    gtk_window_set_transient_for(window, state.window);
    let layout = gtk_box_new(1, 10);
    adw_window_set_content(window, layout);
    let header = adw_header_bar_new();
    adw_header_bar_set_decoration_layout(header, cstring(":close").as_ptr());
    let maximize_button =
        gtk_button_new_from_icon_name(cstring("window-maximize-symbolic").as_ptr());
    gtk_widget_set_tooltip_text(maximize_button, cstring("Maximizar").as_ptr());
    g_signal_connect_data(
        maximize_button,
        cstring("clicked").as_ptr(),
        maximize_preview as *const c_void,
        window,
        std::ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        window,
        cstring("notify::maximized").as_ptr(),
        update_maximize_button as *const c_void,
        maximize_button,
        std::ptr::null_mut(),
        0,
    );
    adw_header_bar_pack_end(header, maximize_button);
    let minimize_button =
        gtk_button_new_from_icon_name(cstring("window-minimize-symbolic").as_ptr());
    gtk_widget_set_tooltip_text(minimize_button, cstring("Minimizar").as_ptr());
    g_signal_connect_data(
        minimize_button,
        cstring("clicked").as_ptr(),
        minimize_preview as *const c_void,
        window,
        std::ptr::null_mut(),
        0,
    );
    adw_header_bar_pack_end(header, minimize_button);
    gtk_box_append(layout, header);
    let picture = gtk_picture_new();
    gtk_picture_set_content_fit(picture, 1);
    gtk_widget_set_hexpand(picture, 1);
    gtk_widget_set_vexpand(picture, 1);
    gtk_widget_set_margin_start(picture, 16);
    gtk_widget_set_margin_end(picture, 16);
    gtk_box_append(layout, picture);
    let status = label("Preparando renderização…", "reset-subtitle");
    gtk_widget_set_margin_start(status, 20);
    gtk_widget_set_margin_end(status, 20);
    gtk_box_append(layout, status);
    let button = gtk_button_new();
    gtk_button_set_child(button, label("CANCELAR", "reset-value"));
    gtk_widget_set_margin_start(button, 16);
    gtk_widget_set_margin_end(button, 16);
    gtk_widget_set_margin_bottom(button, 16);
    gtk_box_append(layout, button);
    let data = (state as *mut UiState).cast();
    g_signal_connect_data(
        button,
        cstring("clicked").as_ptr(),
        cancel_render as *const c_void,
        data,
        std::ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        window,
        cstring("close-request").as_ptr(),
        preview_close as *const c_void,
        data,
        std::ptr::null_mut(),
        0,
    );
    state.preview_window = window;
    state.preview_picture = picture;
    state.preview_status = status;
    state.preview_cancel = button;
    state.preview = Some(preview);
    draw_preview(state);
    gtk_window_present(window);
}
extern "C" fn refresh(data: *mut c_void) -> c_int {
    unsafe {
        let state = &mut *(data as *mut UiState);
        let result = state.running.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(r) => Some(describe_result(r)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some("O benchmark foi interrompido inesperadamente.".into())
            }
        });
        if let Some(result) = result {
            if !state.preview_window.is_null() && state.progress.is_some() {
                draw_preview(state);
                gtk_label_set_text(state.preview_status, cstring(&result).as_ptr());
                gtk_widget_set_sensitive(state.preview_cancel, 0);
            }
            state.running = None;
            state.cancel = None;
            state.progress = None;
            state.benchmark = Some(result);
            if state.closing {
                g_main_loop_quit(state.main_loop);
                return 1;
            }
        }
        render(state);
    }
    1
}
extern "C" fn navigate(_: *mut c_void, data: *mut c_void) {
    unsafe {
        let item = &*(data as *const NavItem);
        let state = &mut *item.state;
        state.page = item.page;
        for &(button, page) in &state.buttons {
            if page == item.page {
                class(button, "active");
            } else {
                gtk_widget_remove_css_class(button, cstring("active").as_ptr());
            }
        }
        render(state);
    }
}
unsafe fn apply_theme(state: &UiState) {
    let dark = state.theme == Theme::Dark;
    adw_style_manager_set_color_scheme(adw_style_manager_get_default(), if dark { 4 } else { 1 });
    gtk_css_provider_load_from_data(
        state.provider,
        cstring(if dark { DARK_CSS } else { LIGHT_CSS }).as_ptr(),
        -1,
    );
    gtk_label_set_text(
        state.theme_label,
        cstring(if dark {
            "☀  Modo claro"
        } else {
            "☾  Modo escuro"
        })
        .as_ptr(),
    );
}
extern "C" fn toggle_theme(_: *mut c_void, data: *mut c_void) {
    unsafe {
        let state = &mut *(data as *mut UiState);
        state.theme = if state.theme == Theme::Dark {
            Theme::Light
        } else {
            Theme::Dark
        };
        apply_theme(state);
        if let Err(e) = save_theme(state.theme) {
            eprintln!("Não foi possível salvar o tema: {e}");
        }
    }
}
fn html_now(benchmark: Option<&str>, theme: Theme) -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    report_html(
        &snapshot(),
        benchmark,
        seconds,
        match theme {
            Theme::Light => ReportTheme::Light,
            Theme::Dark => ReportTheme::Dark,
        },
    )
}
unsafe fn export_message(state: &UiState, message: &str) {
    gtk_label_set_text(state.export_status, cstring(message).as_ptr());
    gtk_widget_set_visible(state.export_status, 1);
}
extern "C" fn export_response(dialog: *mut c_void, response: c_int, data: *mut c_void) {
    unsafe {
        let state = &mut *(data as *mut UiState);
        if response == -3 {
            // GTK_RESPONSE_ACCEPT
            let file = gtk_file_chooser_get_file(dialog);
            if file.is_null() {
                export_message(state, "Não foi possível obter o arquivo selecionado.");
            } else {
                let raw_path = g_file_get_path(file);
                if raw_path.is_null() {
                    export_message(
                        state,
                        "Selecione um arquivo em um sistema de arquivos local.",
                    );
                } else {
                    let path_bytes = std::ffi::CStr::from_ptr(raw_path).to_bytes();
                    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(path_bytes));
                    let mut sections = Vec::new();
                    if let Some(text) = &state.benchmark {
                        sections.push(format!("BENCHMARK\n{text}"));
                    }
                    if state.running.is_some() {
                        sections.push(
                            "Um teste está em andamento; o resultado ainda não está disponível."
                                .into(),
                        );
                    }
                    let session = if sections.is_empty() {
                        None
                    } else {
                        Some(sections.join("\n\n"))
                    };
                    match std::fs::write(path, html_now(session.as_deref(), state.theme)) {
                        Ok(()) => {
                            export_message(state, &format!("Relatório salvo em {}", path.display()))
                        }
                        Err(e) => export_message(
                            state,
                            &format!("Não foi possível salvar o relatório: {e}"),
                        ),
                    }
                    g_free(raw_path.cast());
                }
                g_object_unref(file);
            }
        }
        gtk_native_dialog_destroy(dialog);
        g_object_unref(dialog);
    }
}
extern "C" fn export_clicked(_: *mut c_void, data: *mut c_void) {
    unsafe {
        let state = &*(data as *const UiState);
        let dialog = gtk_file_chooser_native_new(
            cstring("Salvar relatório HTML").as_ptr(),
            state.window,
            1,
            cstring("Salvar").as_ptr(),
            cstring("Cancelar").as_ptr(),
        );
        gtk_file_chooser_set_current_name(dialog, cstring("reset-explode-relatorio.html").as_ptr());
        g_signal_connect_data(
            dialog,
            cstring("response").as_ptr(),
            export_response as *const c_void,
            data,
            std::ptr::null_mut(),
            0,
        );
        gtk_native_dialog_show(dialog);
    }
}
extern "C" fn close(_: *mut c_void, data: *mut c_void) -> c_int {
    unsafe {
        let state = &mut *(data as *mut UiState);
        if state.running.is_some() {
            if let Some(cancel) = &state.cancel {
                cancel.store(true, Ordering::Relaxed);
                state.closing = true;
                return 1;
            }
        }
        g_main_loop_quit(state.main_loop);
    }
    0
}
unsafe fn add_nav(
    sidebar: *mut c_void,
    state: *mut UiState,
    caption: &'static str,
    page: &'static str,
) {
    let button = gtk_button_new();
    class(button, "reset-nav");
    gtk_button_set_child(button, label(caption, "reset-nav-label"));
    if page == "Visão geral" {
        class(button, "active");
    }
    gtk_box_append(sidebar, button);
    (*state).buttons.push((button, page));
    let item = Box::into_raw(Box::new(NavItem { state, page }));
    g_signal_connect_data(
        button,
        cstring("clicked").as_ptr(),
        navigate as *const c_void,
        item.cast(),
        std::ptr::null_mut(),
        0,
    );
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.iter().any(|a| a == "--report") {
        print!("{}", snapshot());
        return;
    }
    if args
        .iter()
        .any(|a| a == "--render-benchmark" || a == "--render-single")
    {
        let threads = if args.iter().any(|a| a == "--render-single") {
            1
        } else {
            std::thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(1)
        };
        match render_bench::run(
            threads,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicUsize::new(0)),
        ) {
            Ok(result) => {
                println!("{}", describe_result(Ok(BenchResult::Render(result))));
                return;
            }
            Err(e) => {
                eprintln!("Erro: {e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(index) = args.iter().position(|a| a == "--export-html") {
        let Some(path) = args.get(index + 1) else {
            eprintln!("Uso: reset-explode-linux --export-html ARQUIVO.html");
            std::process::exit(2)
        };
        if let Err(e) = std::fs::write(path, html_now(None, load_theme())) {
            eprintln!("Falha ao salvar o relatório: {e}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("Nenhuma sessão gráfica encontrada. Use --report para o relatório em texto.");
        std::process::exit(1);
    }
    unsafe {
        g_set_prgname(cstring("reset-explode-linux").as_ptr());
        gtk_init();
        adw_init();
        gtk_window_set_default_icon_name(cstring("reset-explode-linux").as_ptr());
        let provider = gtk_css_provider_new();
        gtk_style_context_add_provider_for_display(gdk_display_get_default(), provider, 600);
        let main_loop = g_main_loop_new(std::ptr::null_mut(), 0);
        let window = adw_window_new();
        gtk_window_set_title(window, cstring("RESET EXPLODE · Linux").as_ptr());
        gtk_window_set_default_size(window, 1120, 760);
        gtk_widget_set_size_request(window, 850, 600);
        let root = gtk_box_new(1, 0);
        adw_window_set_content(window, root);
        let header = adw_header_bar_new();
        gtk_box_append(root, header);
        let export_button = gtk_button_new();
        gtk_button_set_child(
            export_button,
            gtk_label_new(cstring("Exportar HTML").as_ptr()),
        );
        adw_header_bar_pack_end(header, export_button);
        let theme_button = gtk_button_new();
        let theme_label = gtk_label_new(cstring("Modo claro").as_ptr());
        gtk_button_set_child(theme_button, theme_label);
        adw_header_bar_pack_end(header, theme_button);
        let export_status = label("", "reset-subtitle");
        gtk_widget_set_margin_start(export_status, 20);
        gtk_widget_set_visible(export_status, 0);
        gtk_box_append(root, export_status);
        let columns = gtk_box_new(0, 18);
        gtk_widget_set_vexpand(columns, 1);
        gtk_box_append(root, columns);
        let sidebar_shell = gtk_box_new(1, 0);
        gtk_widget_set_size_request(sidebar_shell, 238, -1);
        class(sidebar_shell, "reset-sidebar");
        gtk_widget_set_margin_start(sidebar_shell, 14);
        gtk_widget_set_margin_top(sidebar_shell, 14);
        gtk_widget_set_margin_bottom(sidebar_shell, 14);
        gtk_box_append(columns, sidebar_shell);
        let sidebar_scroll = gtk_scrolled_window_new();
        gtk_widget_set_vexpand(sidebar_scroll, 1);
        gtk_box_append(sidebar_shell, sidebar_scroll);
        let sidebar = gtk_box_new(1, 2);
        gtk_scrolled_window_set_child(sidebar_scroll, sidebar);
        let brand = gtk_box_new(0, 8);
        let logo_data = include_bytes!("../reset-explode-sidebar.png");
        let logo_bytes = g_bytes_new(logo_data.as_ptr().cast(), logo_data.len());
        let logo_texture = gdk_texture_new_from_bytes(logo_bytes, std::ptr::null_mut());
        g_bytes_unref(logo_bytes);
        if !logo_texture.is_null() {
            let logo = gtk_picture_new();
            gtk_picture_set_paintable(logo, logo_texture);
            g_object_unref(logo_texture);
            gtk_picture_set_content_fit(logo, 1);
            gtk_widget_set_size_request(logo, 48, 48);
            gtk_box_append(brand, logo);
        }
        gtk_box_append(brand, label("RESET  EXPLODE", "reset-brand"));
        gtk_box_append(sidebar, brand);
        let scroll = gtk_scrolled_window_new();
        gtk_widget_set_vexpand(scroll, 1);
        gtk_widget_set_hexpand(scroll, 1);
        gtk_widget_set_margin_top(scroll, 20);
        gtk_widget_set_margin_end(scroll, 24);
        gtk_widget_set_margin_bottom(scroll, 20);
        gtk_box_append(columns, scroll);
        let content = gtk_box_new(1, 14);
        gtk_widget_set_margin_start(content, 4);
        gtk_widget_set_margin_end(content, 8);
        gtk_widget_set_margin_bottom(content, 20);
        gtk_scrolled_window_set_child(scroll, content);
        let state = Box::into_raw(Box::new(UiState {
            content,
            page: "Visão geral",
            buttons: Vec::new(),
            benchmark: None,
            closing: false,
            main_loop,
            running: None,
            cancel: None,
            progress: None,
            theme: load_theme(),
            provider,
            theme_label,
            window,
            export_status,
            preview_window: std::ptr::null_mut(),
            preview_picture: std::ptr::null_mut(),
            preview_status: std::ptr::null_mut(),
            preview_cancel: std::ptr::null_mut(),
            preview: None,
        }));
        g_signal_connect_data(
            export_button,
            cstring("clicked").as_ptr(),
            export_clicked as *const c_void,
            state.cast(),
            std::ptr::null_mut(),
            0,
        );
        apply_theme(&*state);
        g_signal_connect_data(
            theme_button,
            cstring("clicked").as_ptr(),
            toggle_theme as *const c_void,
            state.cast(),
            std::ptr::null_mut(),
            0,
        );
        add_nav(sidebar, state, "◉   Visão geral", "Visão geral");
        gtk_box_append(sidebar, label("RESET EXPLODE", "reset-group"));
        for (caption, page) in [
            ("▣   Sistema", "SISTEMA"),
            ("◷   Sensores", "SENSORES"),
            ("▧   Placa de vídeo", "PLACA DE VÍDEO"),
            ("◫   Discos", "ARMAZENAMENTO"),
            ("◇   Energia", "ENERGIA"),
            ("◎   Rede", "REDE"),
        ] {
            add_nav(sidebar, state, caption, page);
        }
        gtk_box_append(sidebar, label("RESET JOGOS", "reset-group"));
        add_nav(sidebar, state, "▦   Biblioteca Steam", "BIBLIOTECA STEAM");
        gtk_box_append(sidebar, label("RESET DESEMPENHO", "reset-group"));
        add_nav(sidebar, state, "◈   Benchmark", "BENCHMARK");
        gtk_box_append(sidebar, label("RESET MAKER", "reset-group"));
        add_nav(sidebar, state, "⌘   Portas seriais", "PORTAS SERIAIS");
        add_nav(sidebar, state, "ⓘ   Sobre", "SOBRE");
        render(&mut *state);
        g_signal_connect_data(
            window,
            cstring("close-request").as_ptr(),
            close as *const c_void,
            state.cast(),
            std::ptr::null_mut(),
            0,
        );
        let timer = g_timeout_add_seconds(3, refresh, state.cast());
        let preview_timer = g_timeout_add_seconds(1, preview_update, state.cast());
        // Distrobox prefixes exported desktop IDs with the container name.
        gtk_widget_realize(window);
        let surface = gtk_native_get_surface(window);
        if !surface.is_null()
            && g_type_check_instance_is_a(surface, gdk_wayland_toplevel_get_type()) != 0
        {
            let id = std::env::var("CONTAINER_ID")
                .ok()
                .filter(|s| {
                    !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                })
                .map(|s| format!("{s}-reset-explode-linux"))
                .unwrap_or_else(|| "reset-explode-linux".into());
            gdk_wayland_toplevel_set_application_id(surface, cstring(&id).as_ptr());
        }
        gtk_window_present(window);
        g_main_loop_run(main_loop);
        g_source_remove(timer);
        g_source_remove(preview_timer);
        g_main_loop_unref(main_loop);
        g_object_unref(provider);
    }
}
