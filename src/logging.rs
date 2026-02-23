use std::fs::File;
use csv::Writer;
use chrono::Local;

pub struct CsvLogger {
    writer: Writer<File>,
    path: String,
}

impl CsvLogger {
    pub fn new() -> anyhow::Result<Self> {
        let path = format!("wattson_{}.csv", Local::now().format("%Y%m%d_%H%M%S"));
        let mut writer = Writer::from_path(&path)?;
        writer.write_record(["timestamp", "frequency_mhz", "power_dbm", "power_w"])?;
        writer.flush()?;
        Ok(Self { writer, path })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn write(&mut self, freq_mhz: f64, dbm: f64) -> anyhow::Result<()> {
        let watts = 10.0f64.powf((dbm - 30.0) / 10.0);
        self.writer.write_record(&[
            Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            format!("{:.1}", freq_mhz),
            format!("{:.2}", dbm),
            format!("{:.6e}", watts),
        ])?;
        self.writer.flush()?;
        Ok(())
    }
}
