use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, Button, CellRendererText, HeaderBar, Label, Orientation,
    Paned, PolicyType, ScrolledWindow, SelectionMode, TextBuffer, TextTagTable, TextView, TreeIter,
    TreeSelection, TreeStore, TreeView, TreeViewColumn,
};

/// Column indices for the tree store.
pub struct TreeColumns;

impl TreeColumns {
    pub const NAME: i32 = 0;
    pub const ID: i32 = 1;
    pub const IS_FOLDER: i32 = 2;
}

/// Named text tags used by the editor.
pub struct Tags;

impl Tags {
    pub const BOLD: &'static str = "bold";
    pub const ITALIC: &'static str = "italic";
    pub const STRIKE: &'static str = "strike";
    pub const CODE: &'static str = "code";
    pub const HEADING1: &'static str = "heading1";
    pub const HEADING2: &'static str = "heading2";
    pub const LIST_ITEM: &'static str = "list-item";
    pub const BLOCKQUOTE: &'static str = "blockquote";
}

pub const BULLET_PREFIX: &str = "• ";

pub struct AppUi {
    pub window: ApplicationWindow,
    pub header: HeaderBar,
    pub open_button: Button,
    pub save_button: Button,
    pub tree_store: TreeStore,
    pub tree_view: TreeView,
    pub tree_selection: TreeSelection,
    pub text_view: TextView,
    pub text_buffer: TextBuffer,
    pub status_label: Label,
    pub bold_button: Button,
    pub italic_button: Button,
    pub strike_button: Button,
    pub code_button: Button,
    pub heading1_button: Button,
    pub heading2_button: Button,
    pub bullet_button: Button,
}

impl AppUi {
    pub fn new(application: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(application)
            .title("Chicken Scratch GTK Editor")
            .default_width(1200)
            .default_height(720)
            .build();

        let header = HeaderBar::builder()
            .title_widget(&Label::new(Some("Chicken Scratch GTK")))
            .show_title_buttons(true)
            .build();

        let open_button = Button::builder()
            .icon_name("document-open-symbolic")
            .tooltip_text("Open .chikn project (Ctrl+O)")
            .build();
        let save_button = Button::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Save current document (Ctrl+S)")
            .build();

        header.pack_start(&open_button);
        header.pack_start(&save_button);

        window.set_titlebar(Some(&header));

        let root = Paned::builder()
            .orientation(Orientation::Horizontal)
            .resize_start_child(true)
            .resize_end_child(true)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .build();

        let tree_store = TreeStore::new(&[
            String::static_type(),
            String::static_type(),
            bool::static_type(),
        ]);

        let tree_view = TreeView::with_model(&tree_store);
        tree_view.set_headers_visible(false);
        tree_view.set_activate_on_single_click(true);

        let renderer = CellRendererText::new();
        let column = TreeViewColumn::new();
        column.pack_start(&renderer, true);
        column.add_attribute(&renderer, "text", TreeColumns::NAME);
        tree_view.append_column(&column);

        let tree_selection = tree_view.selection();
        tree_selection.set_mode(SelectionMode::Single);

        let tree_scroll = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .min_content_width(280)
            .build();
        tree_scroll.set_child(Some(&tree_view));

        let text_view = TextView::new();
        text_view.set_wrap_mode(gtk::WrapMode::WordChar);
        text_view.set_monospace(false);
        text_view.set_top_margin(16);
        text_view.set_bottom_margin(16);
        text_view.set_left_margin(12);
        text_view.set_right_margin(12);

        let text_buffer = text_view.buffer();
        register_text_tags(text_buffer.tag_table());

        let editor_scroll = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .build();
        editor_scroll.set_child(Some(&text_view));

        let toolbar = Box::new(Orientation::Horizontal, 6);
        toolbar.set_margin_start(12);
        toolbar.set_margin_end(12);
        toolbar.set_margin_top(12);
        toolbar.set_margin_bottom(6);

        let (
            bold_button,
            italic_button,
            strike_button,
            code_button,
            heading1_button,
            heading2_button,
            bullet_button,
        ) = create_toolbar_buttons();

        toolbar.append(&bold_button);
        toolbar.append(&italic_button);
        toolbar.append(&strike_button);
        toolbar.append(&code_button);
        toolbar.append(&heading1_button);
        toolbar.append(&heading2_button);
        toolbar.append(&bullet_button);

        let status_bar = Box::new(Orientation::Horizontal, 6);
        status_bar.set_margin_all(8);
        status_bar.add_css_class("statusbar");
        let status_label = Label::new(Some("Ready"));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_hexpand(true);
        status_bar.append(&status_label);

        let editor_stack = Box::new(Orientation::Vertical, 0);
        editor_stack.append(&toolbar);
        editor_stack.append(&editor_scroll);
        editor_stack.append(&status_bar);

        root.set_start_child(Some(&tree_scroll));
        root.set_end_child(Some(&editor_stack));

        window.set_child(Some(&root));

        Self {
            window,
            header,
            open_button,
            save_button,
            tree_store,
            tree_view,
            tree_selection,
            text_view,
            text_buffer,
            status_label,
            bold_button,
            italic_button,
            strike_button,
            code_button,
            heading1_button,
            heading2_button,
            bullet_button,
        }
    }
}

fn create_toolbar_buttons() -> (Button, Button, Button, Button, Button, Button, Button) {
    let bold_button = Button::builder()
        .use_underline(false)
        .tooltip_text("Bold")
        .build();
    bold_button.set_child(Some(&markup_label("<b>B</b>")));

    let italic_button = Button::builder()
        .use_underline(false)
        .tooltip_text("Italic")
        .build();
    italic_button.set_child(Some(&markup_label("<i>I</i>")));

    let strike_button = Button::builder()
        .use_underline(false)
        .tooltip_text("Strikethrough")
        .build();
    strike_button.set_child(Some(&markup_label("<span strikethrough='true'>S</span>")));

    let code_button = Button::builder()
        .use_underline(false)
        .tooltip_text("Inline code")
        .build();
    code_button.set_child(Some(&markup_label("<tt>code</tt>")));

    let heading1_button = Button::builder()
        .label("H1")
        .tooltip_text("Heading 1")
        .build();

    let heading2_button = Button::builder()
        .label("H2")
        .tooltip_text("Heading 2")
        .build();

    let bullet_button = Button::builder().tooltip_text("Bullet list").build();
    bullet_button.set_child(Some(&markup_label("<b>•</b>")));

    (
        bold_button,
        italic_button,
        strike_button,
        code_button,
        heading1_button,
        heading2_button,
        bullet_button,
    )
}

fn markup_label(markup: &str) -> Label {
    let label = Label::new(None);
    label.set_use_markup(true);
    label.set_markup(markup);
    label
}

fn register_text_tags(table: &TextTagTable) {
    if table.lookup(Tags::BOLD).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::BOLD)
                .weight(pango::Weight::Bold)
                .build(),
        );
    }

    if table.lookup(Tags::ITALIC).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::ITALIC)
                .style(pango::Style::Italic)
                .build(),
        );
    }

    if table.lookup(Tags::STRIKE).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::STRIKE)
                .strikethrough(true)
                .build(),
        );
    }

    if table.lookup(Tags::CODE).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::CODE)
                .family("Monospace")
                .weight(pango::Weight::Medium)
                .foreground("#d7ba7d")
                .background("#1e1e1e")
                .build(),
        );
    }

    if table.lookup(Tags::HEADING1).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::HEADING1)
                .weight(pango::Weight::Bold)
                .scale(1.4)
                .pixels_above_lines(12)
                .pixels_below_lines(6)
                .build(),
        );
    }

    if table.lookup(Tags::HEADING2).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::HEADING2)
                .weight(pango::Weight::Semibold)
                .scale(1.2)
                .pixels_above_lines(10)
                .pixels_below_lines(4)
                .build(),
        );
    }

    if table.lookup(Tags::LIST_ITEM).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::LIST_ITEM)
                .indent(24)
                .pixels_above_lines(2)
                .pixels_below_lines(2)
                .build(),
        );
    }

    if table.lookup(Tags::BLOCKQUOTE).is_none() {
        table.add(
            &gtk::TextTag::builder()
                .name(Tags::BLOCKQUOTE)
                .left_margin(24)
                .pixels_above_lines(4)
                .pixels_below_lines(4)
                .style(pango::Style::Italic)
                .build(),
        );
    }
}
