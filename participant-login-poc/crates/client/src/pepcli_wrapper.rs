use std::env::temp_dir;
use std::path::PathBuf;
use std::process::Command;
use std::string::FromUtf8Error;
use log::debug;
use tempfile::tempdir;
use regex;


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

    ///Dowloads all the data available to the user. Data gets stored in an OS-managed temporary directory,
    /// to avoid leaving sensitive files around in case of failures.
    /// See also https://gitlab.pep.cs.ru.nl/pep-public/user-docs/-/wikis/Uploading-and-downloading-data#downloading-data
    pub fn download_all(&self){

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
    pub fn get_columns(&self) -> Result<Vec<&str>, FromUtf8Error> {
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
        let column_vector: Vec<&str> = caps_iterator.map(|cap| cap.as_str()).collect(); //Don't worry, we're not doing any linear Algebra here :D

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

