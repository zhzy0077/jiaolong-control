use anyhow::{Result, ensure};

pub const MODE: u8 = 8;
pub const RPM: u8 = 13;
pub const SWITCH: u8 = 20;
pub const SPEED: u8 = 21;
pub const FAN_MIN: u8 = 22;
pub const FAN_MAX: u8 = 58;

pub fn fan_fraction(speed: u8) -> f32 {
    (speed.clamp(FAN_MIN, FAN_MAX) - FAN_MIN) as f32 / (FAN_MAX - FAN_MIN) as f32
}

pub fn request(command: u8, value: Option<u8>) -> [u8; 32] {
    let mut data = [0; 32];
    data[1] = if value.is_some() { 251 } else { 250 };
    data[3] = command;
    if let Some(value) = value {
        data[4] = value;
    }
    data
}

pub fn rpm(data: &[u8]) -> Result<(u16, u16)> {
    ensure!(
        data.len() >= 8,
        "OutData 长度 {}，转速读取需要 8 字节",
        data.len()
    );
    Ok((
        u16::from_le_bytes([data[4], data[5]]),
        u16::from_le_bytes([data[6], data[7]]),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Performance,
    Balanced,
    Quiet,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Performance, Self::Balanced, Self::Quiet];
    pub fn from_byte(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Balanced),
            1 => Ok(Self::Performance),
            2 => Ok(Self::Quiet),
            _ => anyhow::bail!("固件模式值：{value}"),
        }
    }
    pub fn byte(self) -> u8 {
        match self {
            Self::Balanced => 0,
            Self::Performance => 1,
            Self::Quiet => 2,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Performance => 0,
            Self::Balanced => 1,
            Self::Quiet => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Performance => "狂暴模式",
            Self::Balanced => "游戏模式",
            Self::Quiet => "办公模式",
        }
    }
    pub fn limits(self) -> (u8, u8) {
        match self {
            Self::Performance => (50, 58),
            Self::Balanced => (35, 50),
            Self::Quiet => (22, 35),
        }
    }
    pub fn default_speed(self) -> u8 {
        [53, 43, 27][self.index()]
    }
    // MainWindow.Init_SettingsAsync uses strict lower/upper checks in these two modes.
    pub fn initial_speed(self, value: u8, saved: u8) -> u8 {
        let (min, max) = self.limits();
        let valid = match self {
            Self::Performance => value > min && value <= max,
            Self::Balanced => value > min && value < max,
            Self::Quiet => value >= min && value < max,
        };
        if valid { value } else { saved }
    }
    pub fn slider(self, value: u8) -> u8 {
        let (min, max) = self.limits();
        if (min..=max).contains(&value) {
            ((value - min) as u16 * 100 / (max - min) as u16) as u8
        } else {
            0
        }
    }
    pub fn speed_at_fraction(self, fraction: f32) -> u8 {
        let speed =
            (FAN_MIN as f32 + fraction.clamp(0., 1.) * (FAN_MAX - FAN_MIN) as f32).round() as u8;
        let (min, max) = self.limits();
        speed.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_slider_scale_and_mode_limits() {
        for mode in Mode::ALL {
            let (min, max) = mode.limits();
            assert_eq!(mode.speed_at_fraction(-1.), min);
            assert_eq!(mode.speed_at_fraction(2.), max);
            for speed in min..=max {
                assert_eq!(mode.speed_at_fraction(fan_fraction(speed)), speed);
            }
        }
        assert_eq!(Mode::Quiet.speed_at_fraction(fan_fraction(35)), 35);
        assert_eq!(Mode::Balanced.speed_at_fraction(fan_fraction(35)), 35);
        assert_eq!(Mode::Balanced.speed_at_fraction(fan_fraction(50)), 50);
        assert_eq!(Mode::Performance.speed_at_fraction(fan_fraction(50)), 50);
        assert_eq!(
            Mode::Balanced.speed_at_fraction(fan_fraction(43) + 0.01),
            43
        );
        assert_eq!(
            Mode::Balanced.speed_at_fraction(fan_fraction(43) + 0.02),
            44
        );
    }
    #[test]
    fn original_wire_vectors() {
        let mut get = [0; 32];
        get[1] = 0xfa;
        get[3] = 13;
        assert_eq!(request(RPM, None), get);
        let mut set = [0; 32];
        set[1] = 0xfb;
        set[3] = 20;
        set[4] = 1;
        assert_eq!(request(SWITCH, Some(1)), set);
        set[3] = 21;
        set[4] = 53;
        assert_eq!(request(SPEED, Some(53)), set);
        assert_eq!(
            rpm(&[0, 0, 0, 0, 0x34, 0x12, 0x78, 0x56]).unwrap(),
            (0x1234, 0x5678)
        );
        assert!(rpm(&[0; 7]).is_err());
    }
    #[test]
    fn original_mode_boundaries() {
        for mode in Mode::ALL {
            let (min, max) = mode.limits();
            assert_eq!(mode.slider(min), 0);
            assert_eq!(mode.slider(max), 100);
            assert_eq!(mode.slider(min - 1), 0);
        }
        assert_eq!(Mode::Performance.initial_speed(50, 53), 53);
        assert_eq!(Mode::Performance.initial_speed(58, 53), 58);
        assert_eq!(Mode::Balanced.initial_speed(50, 43), 43);
        assert_eq!(Mode::Quiet.initial_speed(22, 27), 22);
        assert_eq!(Mode::Quiet.initial_speed(35, 27), 27);
    }
}
