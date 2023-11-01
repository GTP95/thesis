#![allow(non_snake_case)]
//! Code adapted from Dioxus' file-explorer example: <https://github.com/DioxusLabs/example-projects>

use std::path::PathBuf;
use dioxus::prelude::*;
use crate::State;
use std::fs::remove_dir_all;

#[derive(PartialEq, Props)]
pub(crate) struct Path {
    pub(crate) path: PathBuf,
}



pub(crate) fn Browser(cx: Scope<Path>) -> Element {
    let status = use_shared_state::<State>(cx).unwrap();
    let path_to_plp_temp_dir=&cx.props.path;

    //Register a callback for when this component gets unmounted to cleanup the temporary directory
    use_on_unmount(cx, {
        to_owned![path_to_plp_temp_dir];
       move|| {
        let _ = remove_dir_all(path_to_plp_temp_dir);   //Ignoring the returned whatever. It can fail, but then it is a temporary directory and will be eventually deleted by the OS anyway
       }
    });

    //pecli creates a subdirectory whose name is the participant's local pseudonym. We need to go one level deeper
    let child_directory=match std::fs::read_dir(path_to_plp_temp_dir){
        Ok(mut iterator) => {
            loop{
                let entry=iterator.next();
                match entry{
                    Some(entry) => {
                        let entry=entry.unwrap();
                        let path=entry.path();
                        if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("GUM"){
                            break path;
                        }
                    },
                    None => {
                        break path_to_plp_temp_dir.to_path_buf();
                    }
                }
            }
        }
        Err(error) => {status.write().current_status=crate::CurrentStatus::Error(error.to_string()); path_to_plp_temp_dir.to_path_buf()}    //Here I have to return something, that's why there is path_to_plp_temp_dir.to_path_buf(). But it doesn't matter as we are transitioning to the error state
    };

    let files = use_ref(cx, || {Files::new(child_directory.to_str().unwrap().to_string())});

    render!(div {
        link { href:"https://fonts.googleapis.com/icon?family=Material+Icons", rel:"stylesheet" }
        style { include_str!("../resources/FileBrowserStyle.css") }
        header {
            i { class: "material-icons icon-menu", "menu" }
            h1 { "Files: " "{files.read().current()}" }
            span { }
            i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
        }
        main {
            files.read().path_names.iter().enumerate().map(|(dir_id, path)| {
                let path_end = path.split('/').last().unwrap_or(path.as_str());
                let icon_type = if path_end.contains('.') {
                    "description"
                } else {
                    "folder"
                };
                rsx! (
                    div { class: "folder", key: "{path}",
                        i { class: "material-icons",
                            onclick: move |_| files.write().enter_dir(dir_id),
                            "{icon_type}"
                            p { class: "cooltip", "0 folders / 0 files" }
                        }
                        h1 { "{path_end}" }
                    }
                )
            })
            files.read().err.as_ref().map(|err| {
                rsx! (
                    div {
                        code { "{err}" }
                        button { onclick: move |_| files.write().clear_err(), "x" }
                    }
                )
            })
        }

    })
}

struct Files {
    path_stack: Vec<String>,
    path_names: Vec<String>,
    err: Option<String>,
}

impl Files {
    fn new(base_path: String) -> Self {
        let mut files = Self {
            path_stack: vec![base_path],
            path_names: vec![],
            err: None,
        };

        files.reload_path_list();

        files
    }

    fn reload_path_list(&mut self) {
        let cur_path = self.path_stack.last().unwrap();
        log::info!("Reloading path list for {:?}", cur_path);
        let paths = match std::fs::read_dir(cur_path) {
            Ok(e) => e,
            Err(err) => {   //Likely we're trying to open a file, so let's open it!
                match open::that(cur_path){
                    Ok(_) => {
                        log::info!("Opened file");
                        return;
                    },
                    Err(err) => {
                        let err = format!("An error occurred: {:?}", err);
                        self.err = Some(err);
                        self.path_stack.pop();
                        return;
                    }
                }

            }
        };
        let collected = paths.collect::<Vec<_>>();
        log::info!("Path list reloaded {:#?}", collected);

        // clear the current state
        self.clear_err();
        self.path_names.clear();

        for path in collected {
            self.path_names
                .push(path.unwrap().path().display().to_string());
        }
        log::info!("path names are {:#?}", self.path_names);
    }

    fn go_up(&mut self) {
        if self.path_stack.len() > 1 {
            self.path_stack.pop();
        }
        self.reload_path_list();
    }

    fn enter_dir(&mut self, dir_id: usize) {
        let path = &self.path_names[dir_id];
        self.path_stack.push(path.clone());
        self.reload_path_list();
    }

    fn current(&self) -> &str {
        self.path_stack.last().unwrap()
    }
    fn clear_err(&mut self) {
        self.err = None;
    }

    ///Changes the base directory from which to start browsing the files.
    /// Intended to be called only once right after the instatiation of the struct.
    fn set_new_base_dir(&mut self, new_cwd: String){
        self.path_stack=vec![new_cwd];
        self.reload_path_list();
    }
}
