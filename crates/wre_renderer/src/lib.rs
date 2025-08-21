#![feature(array_try_from_fn)]

use ash::vk;
use ash_bootstrap::{
    Device, DeviceBuilder, Instance, InstanceBuilder, PhysicalDeviceSelector, PreferredDeviceType,
    QueueType, Swapchain, SwapchainBuilder,
};
use std::sync::Arc;
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

struct FrameData {
    pool: vk::CommandPool,
    buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    render_fence: vk::Fence,
}

const FRAME_OVERLAP: usize = 2;

pub struct WreRenderer {
    instance: Arc<Instance>,
    device: Arc<Device>,
    swapchain: Option<Swapchain>,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    frames: [FrameData; FRAME_OVERLAP],
    current_frame: usize,

    graphics_queue: vk::Queue,
    graphics_queue_family: u32,
}

impl WreRenderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, RendererError> {
        let instance =
            InstanceBuilder::new(Some((window.window_handle()?, window.display_handle()?)))
                .app_name("Example Vulkan Application")
                .engine_name("Example Vulkan Engine")
                .request_validation_layers(true)
                .build()?;

        let features12 = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_indexing(true);

        let features13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let physical_device = PhysicalDeviceSelector::new(instance.clone())
            .preferred_device_type(PreferredDeviceType::Discrete)
            .add_required_extension_feature(features12)
            .add_required_extension_feature(features13)
            .select()?;

        let device = Arc::new(DeviceBuilder::new(physical_device, instance.clone()).build()?);
        let (graphics_queue_family, graphics_queue) = device.get_queue(QueueType::Graphics)?;
        let graphics_queue_family = graphics_queue_family as u32;

        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(graphics_queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let frames = create_frames(device.clone(), command_pool_create_info)?;

        let mut renderer = Self {
            instance,
            device,
            swapchain: None,
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
            graphics_queue,
            graphics_queue_family,
            frames,
            current_frame: 0,
        };

        let size = window.inner_size();
        renderer.init_swapchain(size.width, size.height)?;

        Ok(renderer)
    }

    pub fn init_swapchain(&mut self, width: u32, height: u32) -> Result<(), ash_bootstrap::Error> {
        let swapchain_builder = SwapchainBuilder::new(self.instance.clone(), self.device.clone());
        let swapchain_image_format = vk::Format::B8G8R8A8_UNORM;
        let surface_format = vk::SurfaceFormat2KHR {
            surface_format: vk::SurfaceFormatKHR {
                format: swapchain_image_format,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            },
            ..Default::default()
        };

        let builder = swapchain_builder
            .desired_format(surface_format)
            .desired_present_mode(vk::PresentModeKHR::MAILBOX)
            .desired_size(vk::Extent2D { width, height })
            .add_image_usage_flags(vk::ImageUsageFlags::TRANSFER_DST);

        if let Some(old) = self.swapchain.take() {
            builder.set_old_swapchain(old);
        }

        let swapchain = builder.build()?;
        self.swapchain_images = swapchain.get_images()?;
        self.swapchain_image_views = swapchain.get_image_views()?;
        self.swapchain = Some(swapchain);

        Ok(())
    }

    fn get_current_frame(&self) -> &FrameData {
        &self.frames[self.current_frame % FRAME_OVERLAP]
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        let swapchain = if let Some(s) = &self.swapchain {
            s
        } else {
            return Ok(());
        };

        let frame_data = self.get_current_frame();

        // Wait till the gpu has finished rendering the last frame. Timeout of 1 second
        unsafe {
            self.device
                .wait_for_fences(&[frame_data.render_fence], true, 1000000000)
        }?;

        unsafe { self.device.reset_fences(&[frame_data.render_fence]) }?;

        let aquire_next_image_info = vk::AcquireNextImageInfoKHR::default()
            .semaphore(frame_data.swapchain_semaphore)
            .timeout(1000000000);

        let (swapchain_image_index, _suboptimal) =
            unsafe { swapchain.acquire_next_image2(&aquire_next_image_info) }?;

        // Shorten for ease of use
        let cmd = frame_data.buffer;

        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
        }?;

        let cmd_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.begin_command_buffer(cmd, &cmd_begin_info) }?;

        create_transition_image_barrier(
            self.device.clone(),
            cmd.clone(),
            self.swapchain_images[swapchain_image_index as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        // Make a clear-color from frame number. This will flash with a 120 frame period.
        let clear_value = vk::ClearColorValue {
            float32: [0.0, 0.0, 1.0, 1.0],
        };

        let clear_range =
            vk::ImageSubresourceRange::default().aspect_mask(vk::ImageAspectFlags::COLOR);

        unsafe {
            self.device.cmd_clear_color_image(
                cmd,
                self.swapchain_images[swapchain_image_index as usize],
                vk::ImageLayout::GENERAL,
                &clear_value,
                &[clear_range],
            )
        };

        // Make the swapchain image into presentable mode
        create_transition_image_barrier(
            self.device.clone(),
            cmd,
            self.swapchain_images[swapchain_image_index as usize],
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        // Finalize the command buffer (we can no longer add commands, but it can now be executed)
        unsafe { self.device.end_command_buffer(cmd) }?;

        Ok(())
    }
}

impl Drop for WreRenderer {
    fn drop(&mut self) {
        for frame in &mut self.frames {
            unsafe {
                self.device.destroy_command_pool(frame.pool, None);
            }
        }

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

fn create_frames(
    device: Arc<Device>,
    command_pool_create_info: vk::CommandPoolCreateInfo,
) -> Result<[FrameData; FRAME_OVERLAP], vk::Result> {
    std::array::try_from_fn(|_| {
        let pool = unsafe { device.create_command_pool(&command_pool_create_info, None)? };

        let command_buffer_create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let mut buffers = unsafe { device.allocate_command_buffers(&command_buffer_create_info)? };
        let buffer = buffers
            .pop()
            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?;

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let swapchain_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        let render_semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        let render_fence = unsafe { device.create_fence(&fence_create_info, None) }?;

        Ok(FrameData {
            pool,
            buffer,
            swapchain_semaphore,
            render_semaphore,
            render_fence,
        })
    })
}

fn create_transition_image_barrier(
    device: Arc<Device>,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    current_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    let aspect_mask = if matches!(new_layout, vk::ImageLayout::ATTACHMENT_OPTIMAL) {
        vk::ImageAspectFlags::DEPTH
    } else {
        vk::ImageAspectFlags::COLOR
    };

    let image_barrier = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
        .old_layout(current_layout)
        .new_layout(new_layout)
        .subresource_range(vk::ImageSubresourceRange::default().aspect_mask(aspect_mask))
        .image(image);

    let binding = [image_barrier];
    let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&binding);
    unsafe { device.cmd_pipeline_barrier2(cmd, &dependency_info) };
}
