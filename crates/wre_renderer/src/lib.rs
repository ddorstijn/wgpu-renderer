use std::sync::Arc;

use ash::vk::{
    ColorSpaceKHR, Extent2D, Format, Image, ImageUsageFlags, ImageView,
    PhysicalDeviceVulkan12Features, PhysicalDeviceVulkan13Features, PresentModeKHR,
    SurfaceFormat2KHR, SurfaceFormatKHR,
};
use ash_bootstrap::{
    Device, DeviceBuilder, Instance, InstanceBuilder, PhysicalDeviceSelector, PreferredDeviceType,
    QueueType, Swapchain, SwapchainBuilder,
};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

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
    device: Arc<Device>,
    swapchain: Option<Swapchain>,
    swapchain_images: Vec<Image>,
    swapchain_image_views: Vec<ImageView>,
}

impl WreRenderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, RendererError> {
        let instance =
            InstanceBuilder::new(Some((window.window_handle()?, window.display_handle()?)))
                .app_name("Example Vulkan Application")
                .engine_name("Example Vulkan Engine")
                .request_validation_layers(true)
                .build()?;

        let features12 = PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_indexing(true);

        let features13 = PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let physical_device = PhysicalDeviceSelector::new(instance.clone())
            .preferred_device_type(PreferredDeviceType::Discrete)
            .add_required_extension_feature(features12)
            .add_required_extension_feature(features13)
            .select()?;

        let device = Arc::new(DeviceBuilder::new(physical_device, instance.clone()).build()?);
        let (_graphics_queue_index, _graphics_queue) = device.get_queue(QueueType::Graphics)?;

        let mut renderer = Self {
            instance,
            device,
            swapchain: None,
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
        };

        let size = window.inner_size();
        renderer.init_swapchain(size.width, size.height)?;

        Ok(renderer)
    }

    fn init_swapchain(&mut self, width: u32, height: u32) -> Result<(), ash_bootstrap::Error> {
        let swapchain_builder = SwapchainBuilder::new(self.instance.clone(), self.device.clone());
        let swapchain_image_format = Format::B8G8R8A8_UNORM;
        let surface_format = SurfaceFormat2KHR {
            surface_format: SurfaceFormatKHR {
                format: swapchain_image_format,
                color_space: ColorSpaceKHR::SRGB_NONLINEAR,
            },
            ..Default::default()
        };

        let builder = swapchain_builder
            .desired_format(surface_format)
            .desired_present_mode(PresentModeKHR::MAILBOX)
            .desired_size(Extent2D { width, height })
            .add_image_usage_flags(ImageUsageFlags::TRANSFER_DST);

        if let Some(old) = self.swapchain.take() {
            builder.set_old_swapchain(old);
        }

        let swapchain = builder.build()?;
        self.swapchain_images = swapchain.get_images()?;
        self.swapchain_image_views = swapchain.get_image_views()?;
        self.swapchain = Some(swapchain);

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

impl Drop for WreRenderer {
    fn drop(&mut self) {
        if let Some(s) = &self.swapchain {
            for image_view in self.swapchain_image_views.drain(..) {
                unsafe {
                    self.device.destroy_image_view(image_view, None);
                }
            }

            s.destroy();
        }

        self.device.destroy();
        self.instance.destroy();
    }
}
