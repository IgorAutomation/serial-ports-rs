use glob::glob;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::io::{ BufRead, BufReader };
use std::path::PathBuf;

fn read_hex_int(usb_device_path: Option<PathBuf>, filename: &str) -> u16 {
    read_option_string(usb_device_path, filename).map_or(0, |s| u16::from_str_radix(&s, 16).unwrap_or(0))
}

fn read_option_string(usb_device_path: Option<PathBuf>, filename: &str) -> Option<String> {
    if usb_device_path.is_none() {
        return None;
    }
    let mut pathname = usb_device_path.unwrap();
    pathname.push(filename);

    let mut line = String::new();
    fs::File::open(pathname).map(|f| BufReader::new(f).read_line(&mut line))
                            .map(|_| line.trim().to_owned()).ok()
}

impl ::UsbPortInfo {
    pub fn new(usb_device_path: Option<PathBuf>) -> Self {
        ::UsbPortInfo {
            vid: read_hex_int(usb_device_path.clone(), "idVendor"),
            pid: read_hex_int(usb_device_path.clone(), "idProduct"),
            serial_number: read_option_string(usb_device_path.clone(), "serial"),
            location: usb_device_path.as_ref().and_then(|dp| dp.file_name()).map(OsStr::to_string_lossy).map(Cow::into_owned),
            manufacturer: read_option_string(usb_device_path.clone(), "manufacturer"),
            product: read_option_string(usb_device_path.clone(), "product"),
            interface: read_option_string(usb_device_path.clone(), "interface"),
        }
    }
}

impl ::ListPortInfo {
    fn new(dev_name: PathBuf) -> Option<Self> {

        let basename = dev_name.file_name().map(OsStr::to_string_lossy).map(Cow::into_owned);
        if basename.is_none() {
            return None;
        }
        let basename = basename.unwrap();

        let device_path = fs::canonicalize(format!("/sys/class/tty/{}/device", basename)).ok();

        let subsystem_path = device_path.as_ref().and_then(|dp| {
            let mut subsystem_path = PathBuf::from(dp);
            subsystem_path.push("subsystem");
            fs::canonicalize(subsystem_path).ok()});
        let subsystem:Option<String> = subsystem_path.and_then(|pb| pb.file_name().map(OsStr::to_string_lossy).map(Cow::into_owned));

        if subsystem.as_ref().map(String::as_str) == Some("platform") {
            return None;
        }

        let usb_device_path = device_path.as_ref().and_then(|dp|
            match subsystem.as_ref().map(String::as_str) {
                Some("usb-serial")  => PathBuf::from(dp).parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()),
                Some("usb")         => PathBuf::from(dp).parent().map(|p| p.to_path_buf()),
                _ => None,
            }
        );

        let (port_type, description, hwid) = match subsystem.as_ref().map(String::as_str) {
            Some("usb") | Some("usb-serial") => {
                let info = ::UsbPortInfo::new(usb_device_path.clone());
                (::ListPortType::UsbPort(info.clone()), info.description(&basename), info.hwid())
            },
            Some("pnp") => (::ListPortType::PnpPort,
                            basename.clone(),
                            read_option_string(device_path.clone(), "id").unwrap_or("".to_owned())),
            Some("amba") => (::ListPortType::AmbaPort,
                             basename.clone(),
                             device_path.as_ref().and_then(|dp| dp.file_name()).map(OsStr::to_string_lossy).map(Cow::into_owned).unwrap()),
            _ => (::ListPortType::Unknown, "".to_owned(), "".to_owned()),
        };

        let info = ::ListPortInfo {
            device: dev_name,
            name: basename,
            description: description,
            hwid: hwid,
            port_type: port_type,
        };

        Some(info)
    }
}

impl ::ListPorts {
    pub fn new() -> Self {
        let mut ports = ::ListPorts {
            ports: Vec::new()
        };
        ports.add_ports_matching("/dev/ttyS*");
        ports.add_ports_matching("/dev/ttyUSB*");
        ports.add_ports_matching("/dev/ttyACM*");
        ports.add_ports_matching("/dev/ttyAMA*");
        ports.add_ports_matching("/dev/rfcomm*");
        ports
    }

    fn add_ports_matching(&mut self, pattern: &str) {
        for entry in glob(pattern).unwrap() {
            if let Ok(path_buf) = entry {
                if let Some(info) = ::ListPortInfo::new(path_buf) {
                    self.ports.push(info);
                }
            }
        }
    }
}
