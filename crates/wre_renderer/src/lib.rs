use std::sync::Arc;

use ash::vk::{ColorSpaceKHR, Extent2D, ImageUsageFlags, PresentModeKHR, SurfaceFormatKHR};
use ash_bootstrap::{
    DeviceBuilder, Instance, InstanceBuilder, PhysicalDeviceSelector, PreferredDeviceType,
    QueueType, Swapchain, SwapchainBuilder,
};
use winit::raw_window_handle::HasDisplayHandle;

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("Renderer load error: {0}")]
    VulkanLoadError(#[from] ash::LoadingError),
    #[error("Renderer init error: {0}")]
    VulkanRuntimeError(#[from] ash::vk::Result),
    #[error("Renderer init error: {0}")]
    VulkanBootrstrapError(#[from] ash_bootstrap::Error),
    #[error("Renderer init error: {0}")]
    WinitError(#[from] winit::raw_window_handle::HandleError),
}

pub struct WreRenderer {
    instance: Arc<Instance>,
    device: Arc<ash_bootstrap::Device>,
    swapchain: Option<Swapchain>,
}

impl WreRenderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, RendererError> {
        let instance = InstanceBuilder::new(window.clone())
            .app_name("Example Vulkan Application")
            .engine_name("Example Vulkan Engine")
            .request_validation_layers(true)
            .build()?;

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
        }?;

        let features12 = ash::vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_indexing(true);

        let features13 = ash::vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let physical_device = PhysicalDeviceSelector::new(instance.clone())
            .surface(surface)
            .preferred_device_type(PreferredDeviceType::Discrete)
            .add_required_extension_feature(features12)
            .add_required_extension_feature(features13)
            .select()?;

        let device = Arc::new(DeviceBuilder::new(physical_device, instance.clone()).build()?);
        let (_graphics_queue_index, _graphics_queue) = device.get_queue(QueueType::Graphics)?;

        let renderer = Self {
            instance,
            device,
            swapchain: None,
        };

        let size = window.inner_size();
        renderer.create_swapchain(size.width, size.height);

        Ok(renderer)
    }

    fn create_swapchain(&self, width: u32, height: u32) -> Swapchain {
        let swapchain_builder = SwapchainBuilder::new(self.instance.clone(), self.device.clone());
        let swapchain_image_format = ash::vk::Format::B8G8R8A8_UNORM;

        let swapchain = swapchain_builder
            .desired_format(ash::vk::SurfaceFormat2KHR::default().surface_format(
                SurfaceFormatKHR {
                    format: swapchain_image_format,
                    color_space: ColorSpaceKHR::SRGB_NONLINEAR,
                },
            ))
            .desired_present_mode(PresentModeKHR::FIFO)
            .desired_size(Extent2D { width, height })
            .image_usage_flags(ImageUsageFlags::TRANSFER_DST)
            .build()
            .unwrap();

        let _swapchain_image_format = swapchain.image_format;
        let _swapchain_images = swapchain.get_images();
        let _swapchain_image_views = swapchain.get_image_views();

        swapchain
    }

    pub fn render(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

impl Drop for WreRenderer {
    fn drop(&mut self) {
        //ERROR - Instance destroyed before others
        self.instance.destroy();
        if let Some(s) = &self.swapchain {
            s.destroy();
        }

        self.device.destroy();
    }
}
