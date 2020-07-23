use std::sync::Arc;
use std::path::PathBuf;
use std::fs::{File, read_dir};
use std::{io, thread};
use std::process::Command;
use std::io::prelude::*;
use std::time::Instant;
use rand::Rng;
use std::sync::mpsc;

fn get_corpus() -> Result<Vec<PathBuf>, String>{
    let mut corpus = vec!();                    //create var to hold corpus
    for item in read_dir("corpus/").unwrap() {  //for item in corpus folder
        match item {                            //match item
            Ok(p) => corpus.push(p.path()),     //if item == ok add item to corpus vec
            Err(e) => return Err(e.to_string()) //else return error
        }
    }
    Ok(corpus)                                  //return corpus
}



fn write_file(path: &PathBuf, data: &Vec<u8>) -> Result<(),std::io::Error>{
    let mut buffer = loop {                     //loop until file opened  
        match File::create(path) {              //attempt to open file
            Ok(file) => break file,             //if file opened return file
            Err(e) => match e.raw_os_error() {  //else match error
                Some(32) => continue,           //if Os Error 32 file in use then try again
                _ => return Err(e)              //else if other error return error
            } 
        };
    };
    buffer.write_all(&data)?;                   //write all data to file
    buffer.flush()?;                            //flush buffer
    Ok(())                                      //return ok
}



fn test_bintext(path: &PathBuf) -> io::Result<Option<i32>>{
    let output = Command::new("./target.exe").args(&[path]).status()?;      //run tmp file against bintext and wait to exit
    Ok(output.code())                                                       //return output code
}



fn save_crash(file: &PathBuf, path: &PathBuf) -> Result<(), std::io::Error>{
    let mut f = File::open(file)?;                                          //open file that caused crash
    let mut buffer: Vec<u8> = Vec::new();                                   //create buffer for bytes
    f.read_to_end(&mut buffer)?;                                            //read file into buffer
    write_file(path, &buffer)?;                                             //write file to crashes folder
    Ok(())                                                                  //return ok
}



fn worker(thr_id: usize, corpus: Arc<Vec<Vec<u8>>>, test_tx: std::sync::mpsc::Sender<usize>, crash_tx: std::sync::mpsc::Sender<i32>) -> io::Result<()>{
    let tmp_name = PathBuf::from(format!("tmp_{}.exe",thr_id));                                                             //create tmp file name
    let mut rng = rand::thread_rng();
    let mut input = Vec::new();
    loop {                                                                                                                  //do forever
        let i = rng.gen_range(0, corpus.len());                                                                             //gen index of target program
        input.clear();                                                                                                      //clear input
        input.extend_from_slice(&corpus[i]);                                                                                //copy target program into input
        for _ in 0..8 {                                                                                                     //do 8 times
            let i = rng.gen_range(0, input.len());                                                                          //generate random index
            input[i] = rng.gen::<u8>();                                                                                     //at index write random byte
        }
        write_file(&tmp_name, &input)?;                                                                                     //write modified input to tmp file

        let exit_code = match test_bintext(&PathBuf::from(&tmp_name)) {                                                     //test and match bintext exit code
            Ok(exit_status) => match exit_status {                                                                          //if returned exit code
                Some(i) => i,                                                                                               //exit_code = exit code
                None => panic!("Didn't return exit code"),                                                                  //else panic
            },
            Err(e) => panic!("Returned error {}", e),                                                                       //if returned error panic
        };

        if exit_code != 0 {                                                                                                 //if exit code not 0
            println!("Returned {}", exit_code);                                                                             //print exit code and save tmp file to crash folder
            let crash_name = &PathBuf::from(format!(r"crashes/{}_{}.exe",exit_code,rng.gen::<u32>()));                      //create crash program name
            save_crash(&tmp_name, &crash_name).unwrap_or_else(|e|panic!("Error saving crash file: {}",e));                  //save input that caused crash to crashes folder
            crash_tx.send(exit_code).unwrap_or_else(|e| panic!(format!("Could not send crash_log: {}",e)));                 //send exit code to main thread
        }

        test_tx.send(1).unwrap_or_else(|e| panic!(format!("Could not send loop count: {}",e)));                             //send test increase to main thread  
    }
}



fn main() {
    let mut corpus = vec!();
    for path in get_corpus().unwrap_or_else(|e| panic!("Error getting corpus: {}",e)) {
        let mut f = File::open(path).unwrap_or_else(|e| panic!("Error opening file in corpus: {}",e));  //open target file in corpus
        let mut data: Vec<u8> = Vec::new();                                             //create buffer to hold bytes
        f.read_to_end(&mut data).unwrap_or_else(|e| panic!("Error reading file in corpus: {}",e));  //read file to buffer
        corpus.push(data);
    }
    let corpus: Arc<Vec<Vec<u8>>> = Arc::new(corpus.into_iter().collect());

    let mut crash_log: Vec<i32> = vec!();                                               //create empty crash log

    let start = Instant::now();                                                         //get start time
    let mut total_tests = 0;                                                            //set total amount of tests
    let mut last_tests = 0;                                                             //set last seconds amount of tests
    let mut tps_time = start;                                                           //set start of this second
    let mut tps_avg = vec!();

    let (test_tx, test_rx) = mpsc::channel();                                           //create channel for threads to send test counter inc
    let (crash_tx, crash_rx) = mpsc::channel();                                         //create channel for threads to send crash exit codes
    for thread_id in 0..6 {                                                             //number of threads to create
        let ttx = mpsc::Sender::clone(&test_tx);                                        //set its channel to communicate test increase
        let ctx = mpsc::Sender::clone(&crash_tx);                                       //set its channel to communicate its crash exit codes
        let c = corpus.clone();
        thread::spawn(move || worker(thread_id, c, ttx, ctx));                          //create thread
    }

    loop{                                                                               //do forever
        if tps_time.elapsed().as_secs_f64() >= 1.0 {                                    //if second has passed
            total_tests+=test_rx.try_iter().sum::<usize>();                             //get current test increases, add them up, then add them to total_tests
            crash_rx.try_iter().for_each(|crash| crash_log.push(crash));                //get any crashes and add the exit codes to crash_log
            let mut unique_crash = crash_log.clone();                                   //create copy of crash log and remove duplicates to get unique crashes
            unique_crash.sort();
            unique_crash.dedup();

            tps_avg.push((total_tests - last_tests) as f64);                            //get latest tests per second number
            tps_time = Instant::now();                                                  //get start of this second
            last_tests = total_tests;                                                   //set last seconds tests to current tests
            println!("{:09.2}| avg: {:>5.2} | crashes: {:^5} | unique: {:^5} |  {}",    //display stats
                    start.elapsed().as_secs_f64(), tps_avg.iter().sum::<f64>()/tps_avg.len() as f64, crash_log.len(), unique_crash.len(), total_tests);
        }
    }
}