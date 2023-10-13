use std::env::temp_dir;
use std::path::PathBuf;
use std::process::Command;
use std::string::FromUtf8Error;
use log::debug;
use tempfile::tempdir;

pub struct PepCliWrapper {  //It seems pepcli doesn't get the server's url as a cli parameter!!!!!!!!!!
    path_to_pepcli: String,
    token: String,
    path_to_temp_dir: PathBuf
}

impl PepCliWrapper{
    pub(crate) fn new(path_to_pepcli: String, oauth_token: String) -> PepCliWrapper{
        let temp_dir=temp_dir();
        PepCliWrapper{
            path_to_pepcli,
            token: oauth_token,
            path_to_temp_dir: temp_dir
        }
    }

    pub fn get_file_list(&self){}
    pub fn download_file(&self, file_id: &str){

    }

    pub fn get_columns(&self) -> Result<String, FromUtf8Error> {
        let output=Command::new(&self.path_to_pepcli)
            .arg("--oauth-token")
            .arg(&self.token)
            .arg("query")
            .arg("column-access")
            .output()
            .expect("Error running pepcli");
        debug!("pepcli invocation: {:?} {:?} {:?} {:?} {:?}", &self.path_to_pepcli, "--oauth-token", &self.token, "query", "column-access");
        debug!("pepcli output: {:?}", output);
        let columns_list=String::from_utf8(output.stdout)?;  //Using the unsafe version would have let me spare some error handling, under the assumption that pepcli always prints valid UTF-8 characters. But it would also have forced me to mark blocks as `unsafe` all over the place
        Ok(columns_list)
    }

}

