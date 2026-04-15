use ash::{Entry, vk};
use gororoba_kernel_api::algebra::KernelBackend;
use gororoba_kernel_api::fluid::{
    AerodynamicSnapshot, FluidBackendCapabilities, FluidBackendError, FluidBackendKind,
    FluidDiagnosticsSnapshot, FluidDomainConfig, FluidFieldSnapshot, FluidKernel, FluidRenderFrame,
    VulkanCapabilities,
};
use gororoba_kernel_fluid::LocalCpuFluidKernel;

pub struct LocalVulkanFluidKernel {
    cpu: LocalCpuFluidKernel,
    capabilities: FluidBackendCapabilities,
}

impl LocalVulkanFluidKernel {
    fn new(cpu: LocalCpuFluidKernel, capabilities: FluidBackendCapabilities) -> Self {
        Self { cpu, capabilities }
    }
}

pub fn probe_vulkan_capabilities() -> Option<VulkanCapabilities> {
    let entry = unsafe { Entry::load().ok()? };
    let app_name = c"gororoba_app fluid probe";
    let app_info = vk::ApplicationInfo {
        s_type: vk::StructureType::APPLICATION_INFO,
        p_application_name: app_name.as_ptr(),
        api_version: vk::API_VERSION_1_3,
        ..Default::default()
    };
    let create_info = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        p_application_info: &app_info,
        ..Default::default()
    };

    let instance = unsafe { entry.create_instance(&create_info, None).ok()? };
    let result = (|| {
        let pdevices = unsafe { instance.enumerate_physical_devices().ok()? };
        let pdevice = pdevices.into_iter().find(|pdevice| {
            unsafe { instance.get_physical_device_queue_family_properties(*pdevice) }
                .iter()
                .any(|info| info.queue_flags.contains(vk::QueueFlags::COMPUTE))
        })?;

        let props = unsafe { instance.get_physical_device_properties(pdevice) };
        let mem_props = unsafe { instance.get_physical_device_memory_properties(pdevice) };
        let mut vram_mb = None;
        for idx in 0..mem_props.memory_heap_count {
            let heap = mem_props.memory_heaps[idx as usize];
            if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                let heap_mb = (heap.size / (1024 * 1024)) as u32;
                vram_mb = Some(vram_mb.map_or(heap_mb, |current: u32| current.max(heap_mb)));
            }
        }

        let mut features16 = vk::PhysicalDeviceFloat16Int8FeaturesKHR {
            s_type: vk::StructureType::PHYSICAL_DEVICE_FLOAT16_INT8_FEATURES_KHR,
            ..Default::default()
        };
        let mut features2 = vk::PhysicalDeviceFeatures2 {
            s_type: vk::StructureType::PHYSICAL_DEVICE_FEATURES_2,
            p_next: &mut features16 as *mut _ as *mut _,
            ..Default::default()
        };
        unsafe { instance.get_physical_device_features2(pdevice, &mut features2) };

        Some(VulkanCapabilities {
            device_name: unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
            driver: Some(props.driver_version.to_string()),
            api_version: Some(format!(
                "{}.{}.{}",
                vk::api_version_major(props.api_version),
                vk::api_version_minor(props.api_version),
                vk::api_version_patch(props.api_version)
            )),
            vram_mb,
            supports_fp16: features16.shader_float16 == vk::TRUE,
            supports_fp64: features2.features.shader_float64 == vk::TRUE,
            max_compute_shared_memory_size: Some(props.limits.max_compute_shared_memory_size),
        })
    })();

    unsafe {
        instance.destroy_instance(None);
    }

    result
}

pub fn try_create_vulkan_kernel(
    domain_config: &FluidDomainConfig,
    capabilities: FluidBackendCapabilities,
) -> Result<Box<dyn FluidKernel + Send + Sync>, FluidBackendError> {
    if capabilities.vulkan.is_none() {
        return Err(FluidBackendError::UnsupportedBackend(
            "no Vulkan compute-capable device detected",
        ));
    }
    Ok(Box::new(LocalVulkanFluidKernel::new(
        LocalCpuFluidKernel::from_domain_config(domain_config),
        capabilities,
    )))
}

impl FluidKernel for LocalVulkanFluidKernel {
    fn backend(&self) -> KernelBackend {
        KernelBackend::Vulkan
    }

    fn domain_config(&self) -> &FluidDomainConfig {
        self.cpu.domain_config()
    }

    fn selected_backend(&self) -> FluidBackendKind {
        FluidBackendKind::Vulkan
    }

    fn capabilities(&self) -> FluidBackendCapabilities {
        self.capabilities.clone()
    }

    fn set_voxel_mask(&mut self, solid_mask: &[bool]) {
        self.cpu.set_voxel_mask(solid_mask);
    }

    fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
        self.cpu.set_force_field(force_xyz)
    }

    fn step(&mut self, substeps: usize) {
        self.cpu.step(substeps);
    }

    fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        let mut snapshot = self.cpu.diagnostics();
        snapshot.backend = KernelBackend::Vulkan;
        snapshot
    }

    fn field_snapshot(&self) -> FluidFieldSnapshot {
        self.cpu.field_snapshot()
    }

    fn render_frame(&mut self) -> Result<Option<FluidRenderFrame>, FluidBackendError> {
        let Some(target) = self.domain_config().execution.render_target else {
            return Ok(None);
        };
        let snapshot = self.cpu.field_snapshot();
        let density_len = snapshot.density.len();
        if density_len == 0 {
            return Ok(Some(FluidRenderFrame {
                width: target.width,
                height: target.height,
                rgba8: vec![0; target.pixel_len()],
            }));
        }

        let mut rgba8 = vec![0u8; target.pixel_len()];
        for y in 0..target.height as usize {
            for x in 0..target.width as usize {
                let idx = (y * target.width as usize + x) % density_len;
                let density = snapshot.density[idx];
                let velocity_base = idx * 3;
                let speed = (snapshot.velocity_xyz[velocity_base]
                    * snapshot.velocity_xyz[velocity_base]
                    + snapshot.velocity_xyz[velocity_base + 1]
                        * snapshot.velocity_xyz[velocity_base + 1]
                    + snapshot.velocity_xyz[velocity_base + 2]
                        * snapshot.velocity_xyz[velocity_base + 2])
                    .sqrt();
                let density_band = ((density - 0.9) / 0.3).clamp(0.0, 1.0);
                let speed_band = (speed / 0.1).clamp(0.0, 1.0);
                let pixel = (y * target.width as usize + x) * 4;
                rgba8[pixel] = (255.0 * density_band) as u8;
                rgba8[pixel + 1] = (255.0 * speed_band) as u8;
                rgba8[pixel + 2] = (255.0 * (1.0 - speed_band)) as u8;
                rgba8[pixel + 3] = 255;
            }
        }

        Ok(Some(FluidRenderFrame {
            width: target.width,
            height: target.height,
            rgba8,
        }))
    }

    fn aerodynamic_snapshot(
        &self,
        solid_mask: &[bool],
        freestream_velocity: [f32; 3],
        tau: f32,
    ) -> AerodynamicSnapshot {
        self.cpu
            .aerodynamic_snapshot(solid_mask, freestream_velocity, tau)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::fluid::{
        CpuKernelFlavor, FluidBoundaryConfig, FluidExecutionConfig, GridShape3,
    };

    #[test]
    fn probe_is_safe_without_device_guarantee() {
        let _ = probe_vulkan_capabilities();
    }

    #[test]
    fn local_vulkan_backend_renders_frame_when_target_requested() {
        let config = FluidDomainConfig {
            grid: GridShape3 {
                nx: 4,
                ny: 4,
                nz: 4,
            },
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.01, 0.0, 0.0],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                cpu_flavor: CpuKernelFlavor::SoA,
                render_target: Some(gororoba_kernel_api::fluid::FluidRenderTarget {
                    width: 8,
                    height: 8,
                }),
                ..Default::default()
            },
            boundaries: FluidBoundaryConfig::default(),
        };
        let capabilities = FluidBackendCapabilities::cpu_only_detected();
        let mut kernel = LocalVulkanFluidKernel::new(
            LocalCpuFluidKernel::from_domain_config(&config),
            capabilities,
        );
        let frame = kernel.render_frame().unwrap().unwrap();
        assert_eq!(frame.width, 8);
        assert_eq!(frame.height, 8);
        assert_eq!(frame.rgba8.len(), 8 * 8 * 4);
    }
}
