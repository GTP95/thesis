use std::env::temp_dir;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::string::FromUtf8Error;
use log::debug;
use log::Level::Debug;
use tempfile::tempdir;
use regex;
use serde::__private::de::Content::U8;


///
pub struct PepCliWrapper {  //It seems pepcli doesn't get the server's url as a cli parameter!!!!!!!!!!
    path_to_pepcli: String,
    token: String,
    path_to_temp_dir: PathBuf,
    participant_group: String
}

impl PepCliWrapper{
    pub(crate) fn new(path_to_pepcli: String, oauth_token: String, irma_attribute: String) -> PepCliWrapper{
        let temp_dir=temp_dir();
        PepCliWrapper{
            path_to_pepcli: path_to_pepcli,
            token: oauth_token,
            path_to_temp_dir: temp_dir,
            participant_group: irma_attribute
        }
    }

    ///Downloads all the data available to the user. Data gets stored in an OS-managed temporary directory,
    /// to avoid leaving sensitive files around in case of failures.
    /// See also https://gitlab.pep.cs.ru.nl/pep-public/user-docs/-/wikis/Uploading-and-downloading-data#downloading-data
    /// Returns the path to the temporary directory containing the files
    #[deprecated(note="The docs talk about the --all-accessible switch, but it doesn't exist, so this function doesn't work :(")]
    pub fn download_all(&self) -> Result<PathBuf, Box<dyn Error>> {
        let path_to_plp_temp_dir=self.path_to_temp_dir.join("PLP");
        match std::fs::create_dir(&path_to_plp_temp_dir){
            Ok(_) => {
                let download=Command::new(&self.path_to_pepcli)
                    .arg("--oauth-token")
                    .arg(&self.token)
                    .arg("pull")
                    .arg("--all-accessible")
                    .arg("--output-directory")
                    .arg(&path_to_plp_temp_dir)
                    .output()?;
                debug!("pepcli invocation: {:?} {:?} {:?} {:?} {:?} {:?} {:?}", &self.path_to_pepcli, "--oauth-token", &self.token, "pull", "--all-accessible", "--output-directory", &path_to_plp_temp_dir);
                debug!("pepcli output: {:?}", &download);

                //Now, it happened during testing to get the following error: "error: std::runtime_error: Temporary download directory /tmp/PLP-pending already exists. Specify --force to clear the directory and download anyway". So I'll check for this and in case it happens force the download
                if std::str::from_utf8(&download.stderr)?.contains("Temporary download directory") && std::str::from_utf8(&download.stderr)?.contains("already exists"){
                    debug!("Temporary download directory already exists. Forcing download");
                    let download=Command::new(&self.path_to_pepcli)
                        .arg("--oauth-token")
                        .arg(&self.token)
                        .arg("pull")
                        .arg("--all-accessible")
                        .arg("--output-directory")
                        .arg(&path_to_plp_temp_dir)
                        .arg("--force")
                        .output()?;
                    debug!("pepcli invocation: {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?}", &self.path_to_pepcli, "--oauth-token", &self.token, "pull", "--all-accessible", "--output-directory", &path_to_plp_temp_dir, "--force");
                    debug!("pepcli output: {:?}", &download);
                }

                Ok(path_to_plp_temp_dir)
            },
            Err(error) => {
                if error.kind()==io::ErrorKind::AlreadyExists{  //Temp directory left by a previous run that didn't exit properly
                    debug!("Temp directory already exists. Attempting to remove it");
                    std::fs::remove_dir_all(&path_to_plp_temp_dir)?;
                    debug!("Temp directory removed. Trying again");
                    self.download_all()
                }
                else{
                    return Err(Box::try_from(error).unwrap());
                }
            }
        }

    }

    #[deprecated(note="Docs warn against using the list command: https://gitlab.pep.cs.ru.nl/pep-public/user-docs/-/wikis/Uploading-and-downloading-data#downloading-data")] //So funny when you write a brand-new function and you deprecate it straight away :D
    pub fn get_file_list(&self) -> Result<String, FromUtf8Error> {
        let output=Command::new(&self.path_to_pepcli)
            .arg("--oauth-token")
            .arg(&self.token)
            .arg("list")
            .arg("-C")
            .arg("Visits")  //TODO: read from config file
            .arg("-P")
            .arg(&self.participant_group)
            .output()
            .expect("Error running pepcli");

        debug!("pepcli invocation: {:} --oauth-token {:} list -C Visits -P {:}", &self.path_to_pepcli, &self.token, &self.participant_group);
        debug!("pepcli output: {:?}", output);
        let available_data =String::from_utf8(output.stdout)?;  //Using the unsafe version would have let me spare some error handling, under the assumption that pepcli always prints valid UTF-8 characters. But it would also have forced me to mark blocks as `unsafe` all over the place
        Ok(available_data)
    }

    pub fn download_file(&self, file_id: &str){

    }

    ///Uses pepcli to query for the column-access. Then constructs a vector of strings containing
    /// the column names.
    pub fn get_columns(&self) -> Result<Vec<String>, FromUtf8Error> {
        let output=Command::new(&self.path_to_pepcli)
            .arg("--oauth-token")
            .arg(&self.token)
            .arg("query")
            .arg("column-access")
            .output()
            .expect("Error running pepcli");
        debug!("pepcli invocation: {:?} {:?} {:?} {:?} {:?}", &self.path_to_pepcli, "--oauth-token", &self.token, "query", "column-access");
        debug!("pepcli output: {:?}", output);
        let pepcli_answer =String::from_utf8(output.stdout)?;  //Using the unsafe version would have let me spare some error handling, under the assumption that pepcli always prints valid UTF-8 characters. But it would also have forced me to mark blocks as `unsafe` all over the place

        //Extract the column names from the output using a regex
        let re = regex::Regex::new(r"Visit(\d+).([A-Z]+)").unwrap();
        let caps_iterator = re.find_iter(&pepcli_answer);
        let column_vector: Vec<String> = caps_iterator.map(|cap| cap.as_str().to_string()).collect(); //Don't worry, we're not doing any linear Algebra here :D

        Ok(column_vector)
    }

    ///Queries the user's enrollment. Not very useful, except for debugging.
    pub fn query_enrollment(&self) -> Result<String, FromUtf8Error>{
        let output=Command::new(&self.path_to_pepcli)
            .arg("--oauth-token")
            .arg(&self.token)
            .arg("query")
            .arg("enrollment")
            .output()
            .expect("Error running pepcli");
        let permissions=String::from_utf8(output.stdout)?;  //Using the unsafe version would have let me spare some error handling, under the assumption that pepcli always prints valid UTF-8 characters. But it would also have forced me to mark blocks as `unsafe` all over the place
        Ok(permissions)
    }

}

