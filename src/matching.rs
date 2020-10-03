use super::fuzz;
use dynerr::*;

use std::fs::{read_dir, remove_file};
use std::path::PathBuf;
use std::process::Command;

const TEMP_NAME: &str = "tmp\\iso_file.exe";

///takes the path to a mutated binary and finds its original form in the corpus
fn find(target: &PathBuf) -> DynResult<Option<(PathBuf, Vec<(usize, u8, u8)>)>> {
    let mutated_bytes = fuzz::get_bytes(&PathBuf::from(target))?;
    Ok(read_dir("corpus/")?.find_map(|path| {
            let path = path.ok()?.path();
            let original_bytes = fuzz::get_bytes(&path).unwrap();
            if original_bytes.len() != mutated_bytes.len() {return None}            //if binaries same size
            let res = mutated_bytes.iter().zip(original_bytes)                      //find all differences in binary
                .enumerate().filter_map(|(i, (mutb, orgb))| 
                    if *mutb!=orgb {Some((i, orgb, *mutb))} else {None}
                ).collect::<Vec<(usize, u8, u8)>>();
            if !res.is_empty() && res.len() < 100 {Some((path, res))} else {None}   //return differences if 0<dif>100
        }
    ))
}


///finds the original binary for each mutated binary in crashes/
pub fn get_results() -> DynResult<()> {
    for path in read_dir("crashes/")                                            //get each crash
        .unwrap().map(|b| Ok(b?.path()))
        .collect::<DynResult<Vec<PathBuf>>>()? {
        let res = find(&path)?;                                                 //match the crash to its original file
        if let Some((file, changes)) = res {                                    //display results
            println!("File: {}\nExit Code: {}", 
                file.to_str().unwrap().split("/").nth(1).unwrap(),
                path.to_str().unwrap().split(&['/','_'][..]).nth(1).unwrap()
            );
            for change in changes {
                println!("{:08X}:    {:02X} -> {:02X}",
                    change.0, change.1, change.2
                )
            }
        } else {println!("Couldn't find match for {}", path.to_str().unwrap())}
        println!();
    }
    Ok(())
}









struct Crash {
    name: String,
    bytes: Vec<u8>,
    modified: Vec<(usize, u8, u8)>,
}

impl Crash {
    fn new(name: &str, modified: Vec<(usize,u8,u8)>) -> DynResult<Self> {
        Ok(Self {
            bytes: fuzz::get_bytes(&PathBuf::from(name))?,
            name: name.to_string(),
            modified,
        })
    }
}


fn test_each(binary: &Crash) -> DynResult<Vec<(usize, u8, u8, i32)>> {
    let mut results = Vec::new();
    for mutation in &binary.modified {
        let mut tmp_bin = binary.bytes.clone();
        assert_eq!(tmp_bin[mutation.0], mutation.1);
        tmp_bin[mutation.0] = mutation.2;
        fuzz::write_file(&PathBuf::from(TEMP_NAME), &tmp_bin)?;                                 //write modified input to tmp file
        let exit_code = Command::new("./target.exe")                                                //test binary against bintext and returns exit code
            .args(&[&PathBuf::from(TEMP_NAME)]).status()?.code()
            .unwrap_or_else(||logged_panic!("Didn't return exit code"));
        if exit_code != 0 {
            results.push((mutation.0, mutation.1, mutation.2, exit_code))
        }
    }
    Ok(results)
}



pub fn isolate() -> DynResult<()> {
    let mut crashes = Vec::new();
    for path in read_dir("crashes/")
        .unwrap().map(|b| Ok(b?.path()))
        .collect::<DynResult<Vec<PathBuf>>>()? {
        let res = find(&path)?;
        if let Some(r) = res {
            crashes.push(Crash::new(r.0.to_str().unwrap(), r.1)?)
        }
    }
    for crash in crashes {
        let res = test_each(&crash)?;
        println!("{}:", crash.name);
        for val in res {
            println!("{:08X}: {:02X} -> {:02X}    returned {}",
                val.0, val.1, val.2, val.3
            )
        }
        println!();
    }
    remove_file(TEMP_NAME)?;
    Ok(())
}
