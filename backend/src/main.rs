use std::{thread, time, fs};
use lm_sensors::LMSensors;
use lm_sensors::prelude::*;

use std::time::{SystemTime, UNIX_EPOCH};

use std::fs::{File, OpenOptions};
use std::io::Write;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Error;

use byteorder::{LittleEndian, WriteBytesExt};


trait Measure  {
    fn measure_mut(&mut self, value:u64, timestamp_mills: u128, counter: bool) -> ();
}

struct CounterBasedMeasure {
    // first_value: u64,
    // sum: u128,
    prev_value: u64,
    prev_timestamp_mills: u128,
    t: u32,
    ct_last: u64,
    file_appender: FileAppender,
}

impl Measure for CounterBasedMeasure {
    fn measure_mut(&mut self, cpu_usage:u64, ts_cur: u128, counter: bool) -> () {
        let s = &self;
        let t_ts_prev = 1000*(s.t as u128) - s.prev_timestamp_mills;
        // Экстраполяция значения счетчика в точке T
        let C_LAST = s.prev_value as f64;
        let C_DELTA = (cpu_usage as f64 - s.prev_value as f64);
        let CT_BEFORE_T = ts_cur - s.prev_timestamp_mills;
        let ct = (C_LAST + C_DELTA * t_ts_prev as f64 / CT_BEFORE_T as f64).round();
        if counter && s.ct_last > 0 {
            let vt = ct as u64 - s.ct_last;
            let v = vt as u8;
            self.file_appender.append(v as u8);
            // self.sum += v as u128;
            // let sum = self.sum;
            // let delta = ct as u64 - self.first_value;
            // let d = sum as i128 - delta as i128;
            // println!("{ts_cur}: vt: {vt}, {cpu_usage} sum: {sum}, :delta: {delta}, d: {d}");
            // println!("{ts_cur}: vt: {vt}, {cpu_usage}");
        } else {
            let vt = ct as u64; // for real values
            // self.first_value = self.prev_value;
            let v = vt as u8;
            self.file_appender.append(v as u8);
            // println!("{ts_cur}: vt: {vt}, ct: {ct} {cpu_usage}, C_LAST:{C_LAST}, C_DELTA: {C_DELTA}");
        }
        self.ct_last = ct as u64;
        self.t += 1;
        // CT_LAST>0: VT=CT-CT_LAST, OUT VT
        // CT_LAST = CT
    }
}

struct Temperature {
    sensors: LMSensors,
    state: Vec<CounterBasedMeasure>,
}

impl Temperature {
    fn temperature_sensor(&mut self, ts_cur: u128) {
        let mut n = 0;
        for chip in self.sensors.chip_iter(None) {
            let chip_name = chip;
            for feature in chip.feature_iter() {
                for sub_feature in feature.sub_feature_iter() {
                    let v = format!("{sub_feature}");
                    if v.contains("_input") {
                        if let Ok(value) = sub_feature.value() {
                            let vv = value.raw_value();
                            let mut c = 0;
                            while ts_cur > 1000 * (self.state[n].t as u128) {
                                // CT=C_LAST+(C_CUR-C_LAST)*(T-T_LAST)/(T_CUR-T_LAST)
                                let cpu_usage = vv as u64;
                                self.state[n].measure_mut(cpu_usage, ts_cur, false);
                                c += 1;
                            }
                            // if c>1 {println!("MORE: {c}");}
                            self.state[n].prev_value = vv as u64;
                            self.state[n].prev_timestamp_mills = ts_cur;
                            // self.
                            // file_appenders.push(create_file_appender(format!("{start_time}-temperature-{chip_name}-{feature}.bin")));
                            // println!("{s}: {chip_name}-{feature}: {value}");
                            n += 1;
                        }
                    }
                }
            }
        }
    }
}

fn create_temperature(start_time: u32) -> Temperature {
    let now = get_timestamp_mils();

    // let mut file_appenders=Vec::new();
    let mut sensors = lm_sensors::Initializer::default().initialize().expect("failed to execute");
    let mut vec=Vec::new();
    for chip in sensors.chip_iter(None) {
        let chip_name = chip;
        for feature in chip.feature_iter() {
            for sub_feature in feature.sub_feature_iter() {
                let v = format!("{sub_feature}");
                if v.contains("_input") {
                    if let Ok(value) = sub_feature.value() {
                        // file_appenders.push(create_file_appender(format!("{start_time}-temperature-{chip_name}-{feature}.bin")));
                        // println!("{s}: {chip_name}-{feature}: {value}");
                        let vv = value.raw_value() as u64;
                        vec.push(CounterBasedMeasure{
                            prev_timestamp_mills: now,
                            prev_value: vv,
                            // first_value: vv,
                            // sum: 0,
                            t: (1 + now / 1000) as u32, // todo отбрасывает или оеругляет?
                            ct_last: 0,
                            file_appender: create_file_appender(format!("{start_time}-temperature-{chip_name}-{feature}.bin"))
                        });
                    }
                }
            }
        }
    }
    return Temperature{state:vec, sensors };
}


struct CPUUsage {
    state: Vec<CounterBasedMeasure>,
    num_cores: usize
}

impl CPUUsage {
    fn cpu_usage_sensor(&mut self, ts_cur: u128) {
        let cpu_info = procfs::KernelStats::new().unwrap(); // todo unwrap easy but duty?
        for n in 0..self.num_cores {
            let mut c = 0;
            while ts_cur > 1000 * (self.state[n].t as u128) {
                // CT=C_LAST+(C_CUR-C_LAST)*(T-T_LAST)/(T_CUR-T_LAST)
                let cpu_usage = cpu_info.cpu_time[n].user + cpu_info.cpu_time[n].system;
                self.state[n].measure_mut(cpu_usage, ts_cur, true);
                c += 1;
            }
            // if c>1 {println!("MORE: {c}");}
            self.state[n].prev_value = cpu_info.cpu_time[n].user + cpu_info.cpu_time[n].system;
            self.state[n].prev_timestamp_mills = ts_cur;
        }
    }
}

fn create_cpu_usage(start_time: u32) -> CPUUsage {
    // todo get num cores and array of CounterBasedMeasure
    let cpu_info = procfs::CpuInfo::new().unwrap(); // todo unwrap easy but duty?
    let num_cores = cpu_info.num_cores();
    let cpu_info = procfs::KernelStats::new().unwrap(); // todo unwrap easy but duty?
    let mut vec=Vec::new();
    let now = get_timestamp_mils();
    for n in 0..num_cores {
        vec.push(CounterBasedMeasure{
            prev_timestamp_mills: now,
            prev_value: cpu_info.cpu_time[n].user + cpu_info.cpu_time[n].system,
            // first_value: cpu_info.cpu_time[n].user + cpu_info.cpu_time[n].system,
            // sum: 0,
            t: (1 + now / 1000) as u32, // todo отбрасывает или оеругляет?
            ct_last: 0,
            file_appender: create_file_appender(format!("{start_time}-cpu-{n}.bin"))
        });
    }
    return CPUUsage{state:vec, num_cores};
}

struct CPUFreq {
    state: Vec<CounterBasedMeasure>,
    num_cores: usize,
    min: u32,
    max: u32,
}

impl CPUFreq {
    fn cpu_usage_sensor(&mut self, ts_cur: u128) {
        let cpu_info_ = procfs::CpuInfo::new().unwrap();
        for n in 0..self.num_cores {
            let mut c = 0;
            while ts_cur > 1000 * (self.state[n].t as u128) {
                // CT=C_LAST+(C_CUR-C_LAST)*(T-T_LAST)/(T_CUR-T_LAST)
                let ff = cpu_info_.get_field(n, "cpu MHz").unwrap();
                let my_f: f64 = ff.parse().unwrap();
                let my_int = 1000f64 * my_f as f64;
                let k= (self.max as f64 - self.min as f64) /255f64;
                let v = ((my_int-self.min as f64)/k).round();
                let vv = v as u64;
                let max = self.max;
                // println!("{vv:?}, ts_cur: {ts_cur}m ff: {ff}, max: {max}");

                self.state[n].measure_mut(vv as u64, ts_cur, false);
                c += 1;
            }
            // if c > 1 { println!("MORE: {c}"); }
            let ff = cpu_info_.get_field(n, "cpu MHz").unwrap();
            let my_f: f64 = ff.parse().unwrap();
            let my_int = 1000f64 * my_f as f64;
            let k= (self.max as f64 - self.min as f64) /255f64;
            let v = ((my_int-self.min as f64)/k).round();
            let vv = v as u64;
            self.state[n].prev_value = vv;
            // println!("vv: {vv}");
            self.state[n].prev_timestamp_mills = ts_cur;
        }
    }
}

fn create_cpu_freq(start_time: u32) -> CPUFreq {
    let cpu_info = procfs::CpuInfo::new().unwrap(); // todo unwrap easy but duty?
    let max_s = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        .expect("Should have been able to read the file");
    let max: u32 = max_s.replace("\n", "").parse().unwrap();

    let min_s = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq")
        .expect("Should have been able to read the file");
    let min: u32 = min_s.replace("\n", "").parse().unwrap();

    // println!("{min:?}-{max:?}");
    let num_cores = cpu_info.num_cores();
    let mut state=Vec::new();
    let now = get_timestamp_mils();
    for n in 0..num_cores {
        let ff = cpu_info.get_field(n, "cpu MHz").unwrap();
        let my_f: f32 = ff.parse().unwrap();
        let my_int = my_f as u64;
        state.push(CounterBasedMeasure{
            prev_timestamp_mills: now,
            prev_value: my_int,
            // first_value: my_int,
            // sum: 0,
            t: (1 + now / 1000) as u32, // todo отбрасывает или оеругляет?
            ct_last: 0,
            file_appender: create_file_appender(format!("{start_time}-cpu-freq-{n}.bin"))
        });
    }
    return CPUFreq{state, num_cores, min, max};
}

fn create_file_appender(s: String) -> FileAppender {
    let file_ref = OpenOptions::new().create(true).append(true).open(s).expect("Unable to open file");
    let file_appender = FileAppender{ file_ref };
    return file_appender;
}

struct FileAppender {
    file_ref: File
}

impl FileAppender {
    fn append(&mut self, value: u8) {
        let buf = [value];
        self.file_ref.write_all(&buf).expect("write failed");
    }

    fn append_u32(&mut self, value: u32) {
        let mut result: Vec<u8> = Vec::new();
        let _ = result.write_u32::<LittleEndian>(value);
        self.file_ref.write_all(&result).expect("write failed");
    }
}

fn get_timestamp_mils() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn main() -> Result<(), Error> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        r.store(false, Ordering::SeqCst);
    })
        .expect("Error setting Ctrl-C handler");

    let start_time = (get_timestamp_mils()/1000) as u32;
    let mut file_appender = create_file_appender(format!("start.bin"));
    file_appender.append_u32(start_time);

    let one_sec_duration = time::Duration::from_millis(250);
    let mut cpu_usage = create_cpu_usage(start_time);
    let mut temp = create_temperature(start_time);
    let mut cpu_freq = create_cpu_freq(start_time);
    while running.load(Ordering::SeqCst) {

        thread::sleep(one_sec_duration);

        let time = get_timestamp_mils();

        // let delta = now.elapsed().as_secs();
        // sensors.append_temp(time);
        // temp.append_temp(time);

        cpu_usage.cpu_usage_sensor(time);
        temp.temperature_sensor(time);
        cpu_freq.cpu_usage_sensor(time);
    }
    // todo do after exit stuff
    println!("Gracefully exit!");
    file_appender.append_u32((get_timestamp_mils()/1000) as u32);
    Ok(())
}
