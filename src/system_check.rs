use sysinfo::{Disks, System};

pub struct SystemRequirements {
    pub disk_space: DiskSpaceStatus,
    pub memory: MemoryStatus,
}

pub struct DiskSpaceStatus {
    pub available_gb: f64,
    pub required_gb: f64,
    pub meets_requirement: bool,
}

pub struct MemoryStatus {
    pub total_gb: f64,
    pub required_gb: f64,
    pub meets_requirement: bool,
}

impl SystemRequirements {
    pub fn check() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        // Check disk space (1.5TB = 1536 GB)
        let required_disk_gb = 1536.0;
        let available_gb = get_total_available_space();
        
        // Check RAM (8GB minimum)
        let required_memory_gb = 8.0;
        let total_memory_bytes = sys.total_memory();
        let total_memory_gb = total_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        
        SystemRequirements {
            disk_space: DiskSpaceStatus {
                available_gb,
                required_gb: required_disk_gb,
                meets_requirement: available_gb >= required_disk_gb,
            },
            memory: MemoryStatus {
                total_gb: total_memory_gb,
                required_gb: required_memory_gb,
                meets_requirement: total_memory_gb >= required_memory_gb,
            },
        }
    }
    
    pub fn all_requirements_met(&self) -> bool {
        self.disk_space.meets_requirement && self.memory.meets_requirement
    }
}

fn get_total_available_space() -> f64 {
    // Create a new Disks instance to get disk information
    let disks = Disks::new_with_refreshed_list();
    
    // Sum up available space across all mounted drives
    let mut total_available_bytes: u64 = 0;
    
    for disk in disks.iter() {
        // Only count actual mounted filesystems, not virtual ones
        let mount_point = disk.mount_point().to_string_lossy();
        
        // On macOS, skip certain virtual filesystems
        #[cfg(target_os = "macos")]
        if mount_point.starts_with("/System/Volumes/") && !mount_point.starts_with("/System/Volumes/Data") {
            continue;
        }
        
        // Skip common virtual filesystems on Linux
        #[cfg(target_os = "linux")]
        if mount_point.starts_with("/dev") || mount_point.starts_with("/proc") || 
           mount_point.starts_with("/sys") || mount_point.starts_with("/run") {
            continue;
        }
        
        total_available_bytes += disk.available_space();
    }
    
    // Convert to GB
    total_available_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}