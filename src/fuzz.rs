use dynerr::*;


use std::sync::{Arc, mpsc};
use std::process::Command;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::Instant;
use std::fs::{File, read_dir};
use std::thread;
use rand::Rng;


///writes a buffer of modified bytes to disk
pub fn write_file(path: &PathBuf, data: &Vec<u8>) -> DynResult<()>{
    let mut buffer = loop {                     //loop until file opened
        match File::create(path) {              //attempt to open file
            Ok(file) => break file,             //if file opened return file
            Err(e) => match e.raw_os_error() {  //else match error
                Some(32) => continue,           //if Os Error 32 file in use then try again
                _ => dynerr!(e)                 //else if other error return error
            }
        };
    };
    buffer.write_all(&data)?;
    buffer.flush()?;
    Ok(())
}


///copies a mutated binary to the crash folder
fn save_crash(file: &PathBuf, path: &PathBuf) -> DynResult<()>{
    let mut f = File::open(file)?;          //open file that caused crash
    let mut buffer: Vec<u8> = Vec::new();   //create buffer for bytes
    f.read_to_end(&mut buffer)?;            //read file into buffer
    write_file(path, &buffer)?;             //write file to crashes folder
    Ok(())                                  //return ok
}



///fuzz loop that will continuously generate mutated binaries and test them against bintext
fn worker(
    thr_id: usize,
    corpus: Arc<Vec<Vec<u8>>>,
    test_tx: std::sync::mpsc::Sender<usize>,
    crash_tx: std::sync::mpsc::Sender<i32>
) -> DynResult<()> {
    let tmp_name = PathBuf::from(format!("tmp\\tmp_{}.exe",thr_id));
    let mut rng = rand::thread_rng();
    let mut input = Vec::new();
    loop {
        let i = rng.gen_range(0, corpus.len());                         //gen index of target program
        input.clear();
        input.extend_from_slice(&corpus[i]);                            //copy target program into input
        for _ in 0..8 {                                                 //do 8 times
            let i = rng.gen_range(0, input.len());                      //generate random index
            input[i] = rng.gen::<u8>();                                 //at index write random byte
        }
        write_file(&tmp_name, &input)?;                                 //write modified input to tmp file

        let exit_code = Command::new("./target.exe")                    //test binary against bintext and returns exit code
            .args(&[&PathBuf::from(&tmp_name)]).status()?.code()
            .unwrap_or_else(||logged_panic!("Didn't return exit code"));
        if exit_code != 0 {
            println!("Returned {}", exit_code);                         //print exit code and save tmp file to crash folder
            let crash_name = &PathBuf::from(
                format!(r"crashes/{}_{}.exe",exit_code,rng.gen::<u32>())
            );
            save_crash(&tmp_name, &crash_name)?;                        //save input that caused crash
            crash_tx.send(exit_code)?;                                  //send exit code to main thread
        }

        test_tx.send(1)?;                                               //send test increase to main thread
    }
}


///gets the bytes of a file
pub fn get_bytes(path: &PathBuf) -> DynResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut f = File::open(path)?;
    f.read_to_end(&mut bytes)?;
    Ok(bytes)
}



///parent fuzz loop
pub fn fuzz() -> DynResult<()> {
    let mut corpus = vec!();
    for path in read_dir("corpus/")?
        .map(|b| Ok(b?.path()))
        .collect::<DynResult<Vec<PathBuf>>>()? {
        corpus.push(get_bytes(&path)?);
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
    for thread_id in 0..6 {
        let ttx = mpsc::Sender::clone(&test_tx);                                        //set its channel to communicate test increase
        let ctx = mpsc::Sender::clone(&crash_tx);                                       //set its channel to communicate its crash exit codes
        let c = corpus.clone();
        thread::spawn(move || check!(worker(thread_id, c, ttx, ctx)));
    }

    loop{
        if tps_time.elapsed().as_secs_f64() >= 1.0 {
            total_tests+=test_rx.try_iter().sum::<usize>();                             //get current test increases, add them up, then add them to total_tests
            crash_rx.try_iter().for_each(|crash| crash_log.push(crash));                //get any crashes and add the exit codes to crash_log
            let mut unique_crash = crash_log.clone();                                   //create copy of crash log and remove duplicates to get unique crashes
            unique_crash.sort();
            unique_crash.dedup();

            tps_avg.push((total_tests - last_tests) as f64);                            //display stats
            tps_time = Instant::now();
            last_tests = total_tests;
            println!("{:09.2}| avg: {:>5.2} | crashes: {:^5} | unique: {:^5} |  {}",
                start.elapsed().as_secs_f64(),
                tps_avg.iter().sum::<f64>()/tps_avg.len() as f64,
                crash_log.len(),
                unique_crash.len(),
                total_tests
            );
        }
    }
}