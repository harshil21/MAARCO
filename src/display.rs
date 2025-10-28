use crossterm::{
    cursor::MoveUp,
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::Write;

use nmea::{Nmea, sentences::FixType};

use crate::usb_serial::SensorData;

pub struct Display {
    prev_lines: u16,
}

enum DisplayItem {
    Header(String, Color),
    Divider(String, Color),
    Data(String, String, Option<String>),
}

impl Display {
    pub fn new() -> Self {
        Self { prev_lines: 0 }
    }

    // Private helper to render a list of DisplayItems
    fn render<W: Write>(&mut self, stdout: &mut W, items: &[DisplayItem]) -> std::io::Result<()> {
        let max_len = items
            .iter()
            .filter_map(|item| {
                if let DisplayItem::Data(label, _, _) = item {
                    Some(label.len())
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        let line_count = items.len() as u16;

        if self.prev_lines > 0 {
            queue!(
                stdout,
                MoveUp(self.prev_lines),
                Clear(ClearType::FromCursorDown)
            )?;
        }

        for item in items {
            match item {
                DisplayItem::Header(s, c) | DisplayItem::Divider(s, c) => {
                    queue!(
                        stdout,
                        SetForegroundColor(*c),
                        Print(s),
                        ResetColor,
                        Print("\n")
                    )?;
                }
                DisplayItem::Data(label, value, opt_unit) => {
                    let padded = format!("{:<width$}: ", label, width = max_len);
                    queue!(stdout, Print(padded))?;
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Green),
                        Print(value),
                        ResetColor
                    )?;
                    if let Some(unit) = opt_unit {
                        queue!(
                            stdout,
                            Print(" "),
                            SetForegroundColor(Color::Red),
                            Print(unit),
                            ResetColor
                        )?;
                    }
                    queue!(stdout, Print("\n"))?;
                }
            }
        }

        stdout.flush()?;

        self.prev_lines = line_count;

        Ok(())
    }

    pub fn update_gps<W: Write>(&mut self, stdout: &mut W, parser: &Nmea) -> std::io::Result<()> {
        let sats = parser.satellites();
        let mut avg_snr = 0u32;
        let mut count = 0u32;
        for sat in &sats {
            if let Some(snr) = sat.snr() {
                avg_snr += snr as u32;
                count += 1;
            }
        }
        let avg_snr_value = if count > 0 { avg_snr / count } else { 0 };

        let items = vec![
            DisplayItem::Header("=== GPS DATA ===".to_string(), Color::Yellow),
            DisplayItem::Data(
                "Timestamp".to_string(),
                format!("{:?}", parser.fix_time.unwrap_or_default()),
                None,
            ),
            DisplayItem::Data(
                "Latitude".to_string(),
                format!("{:?}", parser.latitude.unwrap_or_default()),
                None,
            ),
            DisplayItem::Data(
                "Longitude".to_string(),
                format!("{:?}", parser.longitude.unwrap_or_default()),
                None,
            ),
            DisplayItem::Data(
                "Altitude".to_string(),
                format!("{:?}", parser.altitude.unwrap_or_default()),
                Some("m".to_string()),
            ),
            DisplayItem::Data(
                "Fix Type".to_string(),
                format!("{:?}", parser.fix_type.unwrap_or_else(|| FixType::Simulation)),
                None,
            ),
            DisplayItem::Data(
                "Speed".to_string(),
                format!("{:?}", parser.speed_over_ground.unwrap_or_default()),
                Some("km/h".to_string()),
            ),
            DisplayItem::Data(
                "Number of Satellites".to_string(),
                format!("{:?}", parser.num_of_fix_satellites.unwrap_or_default()),
                None,
            ),
            DisplayItem::Data("HDOP".to_string(), format!("{:?}", parser.hdop.unwrap_or_default()), None),
            DisplayItem::Data("VDOP".to_string(), format!("{:?}", parser.vdop.unwrap_or_default()), None),
            DisplayItem::Data("PDOP".to_string(), format!("{:?}", parser.pdop.unwrap_or_default()), None),
            DisplayItem::Divider("=================".to_string(), Color::Yellow),
            DisplayItem::Data(
                "Avg SNR".to_string(),
                format!("{:?}", avg_snr_value),
                Some("db-Hz".to_string()),
            ),
        ];

        self.render(stdout, &items)
    }

    pub fn update_arduino<W: Write>(
        &mut self,
        stdout: &mut W,
        arduino_data: &SensorData,
    ) -> std::io::Result<()> {
        let items = vec![
            DisplayItem::Header("=== Arduino Sensor Data ===".to_string(), Color::Yellow),
            DisplayItem::Data(
                "Sonar".to_string(),
                format!("{:?}", arduino_data.sonar_mm),
                Some("mm".to_string()),
            ),
            DisplayItem::Data(
                "ToF1".to_string(),
                format!("{:?}", arduino_data.tof1_mm),
                Some("mm".to_string()),
            ),
            DisplayItem::Data(
                "ToF2".to_string(),
                format!("{:?}", arduino_data.tof2_mm),
                Some("mm".to_string()),
            ),
            DisplayItem::Data(
                "Current Motor 1".to_string(),
                format!("{:?}", arduino_data.current_motor_1_ma),
                Some("A".to_string()),
            ),
            DisplayItem::Data(
                "Current Motor 2".to_string(),
                format!("{:?}", arduino_data.current_motor_2_ma),
                Some("A".to_string()),
            ),
            // DisplayItem::Data(
            //     "Roll".to_string(),
            //     format!("{:?}", arduino_data.roll),
            //     Some("°".to_string()),
            // ),
            // DisplayItem::Data(
            //     "Pitch".to_string(),
            //     format!("{:?}", arduino_data.pitch),
            //     Some("°".to_string()),
            // ),
            // DisplayItem::Data(
            //     "Yaw".to_string(),
            //     format!("{:?}", arduino_data.yaw),
            //     Some("°".to_string()),
            // ),
            DisplayItem::Data(
                "Motor 1 RPM".to_string(),
                format!("{:?}", arduino_data.motor_1_rpm),
                None,
            ),
            DisplayItem::Data(
                "Motor 2 RPM".to_string(),
                format!("{:?}", arduino_data.motor_2_rpm),
                None,
            ),
            DisplayItem::Data(
                "Motor 1 Total Rotations".to_string(),
                format!("{:?}", arduino_data.motor_1_tot_rotations),
                None,
            ),
            DisplayItem::Data(
                "Motor 2 Total Rotations".to_string(),
                format!("{:?}", arduino_data.motor_2_tot_rotations),
                None,
            ),
        ];

        self.render(stdout, &items)
    }
}
