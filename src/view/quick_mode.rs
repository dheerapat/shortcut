mod imp {
    use std::cell::{OnceCell, RefCell};

    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    use adw::traits::{BinExt, EntryRowExt};

    use gtk::glib::subclass::InitializingObject;
    use gtk::glib::{self, clone, closure, Properties, Sender};
    use gtk::prelude::{Cast, CastNone, FileExt, GObjectPropertyExpressionExt, ObjectExt};
    use gtk::subclass::prelude::*;
    use gtk::traits::{EditableExt, WidgetExt};
    use gtk::{ClosureExpression, CompositeTemplate};

    use crate::component::viewport::Action;
    use crate::model::Desktop;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::QuickMode)]
    #[template(resource = "/io/github/andreibachim/shortcut/quick_mode.ui")]
    pub struct QuickMode {
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "exec", get, set, type = String, member = exec)]
        #[property(name = "icon", get, set, type = String, member = icon)]
        pub data: RefCell<Desktop>,
        pub sender: OnceCell<Sender<Action>>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub name_input: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub name_preview: TemplateChild<gtk::Label>,
        #[template_child]
        pub exec_input: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub icon_input: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub icon_preview: TemplateChild<adw::Bin>,
    }

    #[gtk::template_callbacks]
    impl QuickMode {
        #[template_callback]
        fn save(&self) {
            let data = self.data.borrow();
            let file_path = gtk::glib::home_dir().join(format!(
                ".local/share/applications/{}.desktop",
                data.name.replace(' ', "-").to_lowercase()
            ));
            let mut file = File::create(file_path).expect("Could not create a new file");
            file.write_all(
                data.get_output()
                    .expect("Could not serialize desktop file for writing")
                    .as_bytes(),
            )
            .expect("Could not write to .desktop file.");
            let _ = self
                .sender
                .get()
                .expect("Could not get sender")
                .send(Action::Completed);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QuickMode {
        const NAME: &'static str = "QuickMode";
        type Type = super::QuickMode;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                data: RefCell::new(Desktop::new()),
                ..Default::default()
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("back", None, move |quick_mode, _, _| {
                let imp = quick_mode.imp();
                let _ = imp.sender.get().unwrap().send(Action::Landing(true));
            });

            klass.install_action("cancel", None, move |quick_mode, _, _| {
                let _ = quick_mode
                    .imp()
                    .sender
                    .get()
                    .unwrap()
                    .send(Action::Landing(true));
            });

            klass.install_action("pick_exec", None, move |quick_mode, _, _| {
                let imp = quick_mode.imp();

                let filters_store = gtk::gio::ListStore::new::<gtk::FileFilter>();
                let executable_filter = gtk::FileFilter::new();
                executable_filter.set_name(Some("Executable files"));
                executable_filter.add_mime_type("application/x-executable");
                filters_store.append(&executable_filter);

                let all_filter = gtk::FileFilter::new();
                all_filter.add_pattern("*");
                all_filter.set_name(Some("All files"));
                filters_store.append(&all_filter);

                let dialog = gtk::FileDialog::builder()
                    .filters(&filters_store)
                    .modal(true)
                    .title("Select Executable File")
                    .build();
                dialog.open(
                    quick_mode
                        .parent()
                        .and_downcast_ref::<adw::ApplicationWindow>(),
                    None::<&gtk::gio::Cancellable>,
                    clone!(@weak imp => move |file| {
                        if let Ok(file) = file {
                            imp.exec_input.set_text(
                                file.path()
                                    .expect("Invalid file path")
                                    .to_str()
                                    .expect("Path is not UTF-8 compliant"),
                            );
                            imp.exec_input.emit_by_name::<()>("apply", &[]);
                        }
                    }),
                );
            });
            klass.install_action("pick_icon", None, move |quick_mode, _, _| {
                let imp = quick_mode.imp();

                let filters_store = gtk::gio::ListStore::new::<gtk::FileFilter>();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Image files"));
                filter.add_mime_type("image/svg+xml");
                filter.add_mime_type("image/png");
                filters_store.append(&filter);

                let file_dialog = gtk::FileDialog::builder()
                    .filters(&filters_store)
                    .title("Select Icon File")
                    .modal(true)
                    .build();
                file_dialog.open(
                    None::<&gtk::Window>,
                    None::<&gtk::gio::Cancellable>,
                    clone!(@weak imp => move |file| {
                        if let Ok(file) = file {
                            imp.icon_input.set_text(
                                file.path()
                                    .expect("Could not extract path from file")
                                    .to_str()
                                    .expect("Path is not UTF-8 compliant"),
                            );
                            imp.icon_input.emit_by_name::<()>("apply", &[]);
                        }
                    }),
                );
            });
        }
        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for QuickMode {
        fn constructed(&self) {
            self.parent_constructed();
            bind_name_preview(self);
            setup_form_validation(self);
            self.icon_input
                .settings()
                .set_gtk_entry_select_on_focus(false);
            self.exec_input
                .settings()
                .set_gtk_entry_select_on_focus(false);
        }
    }

    fn bind_name_preview(slf: &QuickMode) {
        slf.name_input
            .bind_property("text", &slf.name_preview.get(), "label")
            .sync_create()
            .transform_to(|_, value: &str| -> Option<&str> {
                match value.is_empty() {
                    true => Some("Preview"),
                    false => Some(value),
                }
            })
            .build();
        slf.name_input
            .bind_property("text", &slf.name_preview.get(), "opacity")
            .sync_create()
            .transform_to(|_, value: &str| -> Option<f64> {
                match value.is_empty() {
                    true => Some(0.3),
                    false => Some(1.0),
                }
            })
            .build();
    }

    fn setup_form_validation(slf: &QuickMode) {
        slf.name_input
            .bind_property("text", slf.obj().as_ref(), "name")
            .sync_create()
            .build();

        slf.exec_input
        .connect_apply(clone!(@weak slf => move |entry_row| {
            let text = entry_row.text();
            let path = Path::new(&text);
            if path.exists() && path.is_file() {
                entry_row.set_css_classes(&[]);
                slf.obj().set_exec(text);
                slf.save_button.grab_focus();
            } else {
                let _ = slf.sender.get().expect("Could not get sender")
                    .send(Action::ShowToast("The executable path is not valid".to_owned(), entry_row.clone().dynamic_cast().unwrap()));
                entry_row.set_css_classes(&["error"]);
            }
        }));

        slf.icon_input
        .connect_apply(clone!(@weak slf => move |entry_row| {
            let text = entry_row.text();
            let path = Path::new(&text);
            if path.exists() && path.is_file() {
                entry_row.set_css_classes(&[]);
                slf.obj().set_icon(text);
                slf.icon_preview.set_child(
                    Some(&gtk::Image::builder().file(entry_row.text()).pixel_size(128).css_classes(vec!["icon-dropshadow"]).build())
                );
                slf.exec_input.grab_focus();
            } else {
                let _ = slf.sender.get().expect("Could not get sender")
                    .send(Action::ShowToast("The icon path is not valid".to_owned(), entry_row.clone().dynamic_cast().unwrap()));
                entry_row.set_css_classes(&["error"]);
            }
        }));

        let name_expression = slf.obj().property_expression("name");
        let exec_expression = slf.obj().property_expression("exec");
        let icon_expression = slf.obj().property_expression("icon");
        ClosureExpression::new::<bool>(
            [&name_expression, &exec_expression, &icon_expression],
            closure!(|_: <QuickMode as ObjectSubclass>::Type,
                      name: String,
                      exec: String,
                      icon: String| {
                !(name.is_empty() || exec.is_empty() || icon.is_empty())
            }),
        )
        .bind(
            &slf.save_button.get(),
            "sensitive",
            Some(slf.obj().as_ref()),
        );
    }

    impl WidgetImpl for QuickMode {}
    impl BoxImpl for QuickMode {}
}

use adw::traits::BinExt;
use glib::Object;
use gtk::{
    glib::{self, Sender},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{EditableExt, WidgetExt},
};

use crate::component::viewport::Action;

glib::wrapper! {
    pub struct QuickMode(ObjectSubclass<imp::QuickMode>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl QuickMode {
    pub fn new(sender: Sender<Action>) -> Self {
        let slf = Object::builder::<Self>().build();
        slf.set_sensitive(false);
        let _ = slf.imp().sender.set(sender);
        slf
    }

    pub fn clear_data(&self) {
        let imp = self.imp();
        imp.name_input.set_text("");
        imp.name_input.grab_focus();
        imp.exec_input.set_text("");
        imp.icon_input.set_text("");
        imp.icon_preview.set_child(Some(
            &gtk::Image::builder()
                .icon_name("preview-placeholder")
                .pixel_size(128)
                .build(),
        ));
    }
}
