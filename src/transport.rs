use anyhow::Result;

#[cfg(windows)]
pub struct Transport(wmi::WMIConnection);

#[cfg(windows)]
impl Transport {
    pub fn new() -> Result<Self> {
        Ok(Self(wmi::WMIConnection::with_namespace_path("root\\WMI")?))
    }
    pub fn execute(&mut self, data: [u8; 32]) -> Result<Vec<u8>> {
        const PATH: &str = "MICommonInterface.InstanceName='ACPI\\PNP0C14\\MIFS_0'";
        let parameters = self
            .0
            .get_object(PATH)?
            .get_method("MiInterface")?
            .ok_or_else(|| anyhow::anyhow!("MiInterface 参数定义缺失"))?
            .spawn_instance()?;
        parameters.put_property("InData", data.to_vec())?;
        let output = self
            .0
            .exec_method(PATH, "MiInterface", Some(&parameters))?
            .ok_or_else(|| anyhow::anyhow!("MiInterface 输出缺失"))?;
        Ok(output.get_property("OutData")?.try_into()?)
    }
}

#[cfg(target_os = "linux")]
pub struct Transport(std::fs::File);

#[cfg(target_os = "linux")]
impl Transport {
    pub fn new() -> Result<Self> {
        use anyhow::Context;
        Ok(Self(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/jiaolong-mifs")
                .context(
                    "打开 /dev/jiaolong-mifs，请加载 linux/jiaolong_mifs.ko 并应用 udev 规则",
                )?,
        ))
    }
    pub fn execute(&mut self, mut data: [u8; 32]) -> Result<Vec<u8>> {
        use std::os::fd::AsRawFd;
        const MIFS_CALL: libc::c_ulong = 0xc0204a01;
        // The ioctl transfers exactly 32 writable bytes for the duration of this call.
        let result = unsafe { libc::ioctl(self.0.as_raw_fd(), MIFS_CALL, data.as_mut_ptr()) };
        if result < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(data.to_vec())
    }
}
