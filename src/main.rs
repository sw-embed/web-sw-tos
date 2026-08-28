fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<web_sw_tos::App>::new().render();
}
