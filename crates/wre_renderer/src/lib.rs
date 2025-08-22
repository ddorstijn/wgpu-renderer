#![feature(array_try_from_fn)]

use ash::vk;
use ash_bootstrap::{
    Device, DeviceBuilder, Instance, PhysicalDeviceSelector, PreferredDeviceType, QueueType,
    Swapchain, SwapchainBuilder,
};
use std::sync::Arc;
use vk_mem::{Alloc, AllocatorCreateInfo};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::vk_util::{copy_image_to_image, transition_image};

mod vk_util;

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

#[derive(Debug)]
struct FrameData {
    pool: vk::CommandPool,
    buffer: vk::CommandBuffer,
    swapchain_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore,
    render_fence: vk::Fence,
}

const FRAME_OVERLAP: usize = 3;

struct AllocatedImage {
    image: vk::Image,
    image_view: vk::ImageView,
    allocation: vk_mem::Allocation,
    image_extent: vk::Extent3D,
    image_format: vk::Format,
}

pub struct WreRenderer {
    frame_number: usize,
    instance: Arc<Instance>,
    device: Arc<Device>,
    window_extent: vk::Extent2D,
    swapchain: Option<Swapchain>,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    graphics_queue: vk::Queue,
    frames: [FrameData; FRAME_OVERLAP],

    allocator: vk_mem::Allocator,
    draw_image: AllocatedImage,
}

impl WreRenderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, RendererError> {
        let instance = ash_bootstrap::InstanceBuilder::new(Some((
            window.window_handle()?,
            window.display_handle()?,
        )))
        .app_name("Example Vulkan Application")
        .engine_name("Example Vulkan Engine")
        .request_validation_layers(true)
        .require_api_version(vk::make_api_version(0, 1, 3, 0))
        .build()?;

        let features12 = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_indexing(true);

        let features13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let physical_device_wrapper = PhysicalDeviceSelector::new(instance.clone())
            .preferred_device_type(PreferredDeviceType::Discrete)
            .add_required_extension_feature(features12)
            .add_required_extension_feature(features13)
            .select()?;

        let vk_instance: &ash::Instance = (*instance).as_ref();
        let vk_physical_device: vk::PhysicalDevice = *physical_device_wrapper.as_ref();

        let device =
            Arc::new(DeviceBuilder::new(physical_device_wrapper, instance.clone()).build()?);

        let allocator_info =
            AllocatorCreateInfo::new(vk_instance, device.as_ref(), vk_physical_device.clone());

        let allocator = unsafe { vk_mem::Allocator::new(allocator_info) }?;

        let (graphics_queue_family, graphics_queue) = device.get_queue(QueueType::Graphics)?;
        let graphics_queue_family = graphics_queue_family as u32;

        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(graphics_queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let frames = create_frames(device.clone(), command_pool_create_info)?;

        let window_size = window.inner_size();
        let window_extent = vk::Extent2D::default()
            .width(window_size.width)
            .height(window_size.height);

        let draw_image_format = vk::Format::R16G16B16A16_SFLOAT;
        let draw_image_usages = vk::ImageUsageFlags::default()
            | vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let draw_image_extent = vk::Extent3D::default()
            .width(window_extent.width)
            .height(window_extent.height)
            .depth(1);

        let rimg_info = vk::ImageCreateInfo::default()
            .format(draw_image_format)
            .usage(draw_image_usages)
            .extent(draw_image_extent)
            .mip_levels(1)
            .array_layers(1)
            .image_type(vk::ImageType::TYPE_2D)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL);

        // For the draw image, we want to allocate it from gpu local memory
        let rimg_allocinfo = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ..Default::default()
        };
        // Allocate and create the image
        let (draw_image_image, draw_image_allocation) =
            unsafe { allocator.create_image(&rimg_info, &rimg_allocinfo) }?;

        // Build a image-view for the draw image to use for rendering
        let rview_info = vk::ImageViewCreateInfo::default()
            .format(draw_image_format)
            .image(draw_image_image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS)
                    .level_count(vk::REMAINING_MIP_LEVELS),
            );
        let draw_image_view = unsafe { device.create_image_view(&rview_info, None) }?;

        let draw_image = AllocatedImage {
            image: draw_image_image,
            image_view: draw_image_view,
            allocation: draw_image_allocation,
            image_extent: draw_image_extent,
            image_format: draw_image_format,
        };

        let mut renderer = Self {
            frame_number: 0,
            instance,
            device,
            swapchain: None,
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
            graphics_queue,
            frames,
            allocator,
            draw_image,
            window_extent,
        };

        renderer.init_swapchain()?;

        Ok(renderer)
    }

    pub fn init_swapchain(&mut self) -> Result<(), ash_bootstrap::Error> {
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
            .desired_present_mode(vk::PresentModeKHR::FIFO)
            .desired_size(
                vk::Extent2D::default()
                    .width(self.window_extent.width)
                    .height(self.window_extent.height),
            )
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

    pub fn set_render_size(&mut self, width: u32, height: u32) {
        self.window_extent = vk::Extent2D::default().width(width).height(height);
    }

    fn get_current_frame(&self) -> &FrameData {
        &self.frames[self.frame_number % FRAME_OVERLAP]
    }

    fn draw_background(&self, cmd: vk::CommandBuffer) {
        //make a clear-color from frame number. This will flash with a 120 frame period.
        let flash = (self.frame_number as f32 / 120.0).sin().abs();
        let clear_value = vk::ClearColorValue {
            float32: [0.0, 0.0, flash, 1.0],
        };

        let clear_ranges = [vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
            .level_count(vk::REMAINING_MIP_LEVELS)];

        //clear image
        unsafe {
            self.device.cmd_clear_color_image(
                cmd,
                self.draw_image.image,
                vk::ImageLayout::GENERAL,
                &clear_value,
                &clear_ranges,
            );
        }
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        let swapchain = if let Some(s) = &self.swapchain {
            s
        } else {
            return Ok(());
        };

        let frame = self.get_current_frame();
        // Wait till the gpu has finished rendering the last frame. Timeout of 1 second
        unsafe {
            self.device
                .wait_for_fences(&[frame.render_fence], true, 1000000000)
        }?;

        unsafe { self.device.reset_fences(&[frame.render_fence]) }?;

        let (swapchain_image_index, _suboptimal) = unsafe {
            swapchain.acquire_next_image(
                *swapchain.as_ref(),
                10000000000,
                frame.swapchain_semaphore,
                vk::Fence::null(),
            )
        }?;

        // Shorten for ease of use
        let cmd = frame.buffer;

        unsafe {
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
        }?;

        let cmd_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.begin_command_buffer(cmd, &cmd_begin_info) }?;

        // we will overwrite it all so we dont care about what was the older layout
        transition_image(
            &self.device,
            cmd.clone(),
            self.draw_image.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        self.draw_background(cmd);

        // Transition the draw image and the swapchain image into their correct transfer layouts
        transition_image(
            &self.device,
            cmd,
            self.draw_image.image,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );
        transition_image(
            &self.device,
            cmd,
            self.swapchain_images[swapchain_image_index as usize],
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        // execute a copy from the draw image into the swapchain
        copy_image_to_image(
            &self.device,
            cmd,
            self.draw_image.image,
            self.swapchain_images[swapchain_image_index as usize],
            self.window_extent,
            self.window_extent,
        );

        // set swapchain image layout to Present so we can show it on the screen
        transition_image(
            &self.device,
            cmd,
            self.swapchain_images[swapchain_image_index as usize],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        // Finalize the command buffer (we can no longer add commands, but it can now be executed)
        unsafe { self.device.end_command_buffer(cmd) }?;

        let buffer_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(cmd)];
        let wait_infos = [vk::SemaphoreSubmitInfo::default()
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
            .semaphore(frame.swapchain_semaphore)];
        let signal_infos = [vk::SemaphoreSubmitInfo::default()
            .stage_mask(vk::PipelineStageFlags2::ALL_GRAPHICS)
            .semaphore(frame.render_semaphore)];

        let submit_info = vk::SubmitInfo2::default()
            .command_buffer_infos(&buffer_infos)
            .signal_semaphore_infos(&signal_infos)
            .wait_semaphore_infos(&wait_infos);

        unsafe {
            self.device
                .queue_submit2(self.graphics_queue, &[submit_info], frame.render_fence)
        }?;

        let wait_semaphores = [frame.render_semaphore];
        let image_indices = [swapchain_image_index];
        let swapchains = [*swapchain.as_ref()];

        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices);

        unsafe { swapchain.queue_present(self.graphics_queue, &present_info) }?;

        self.frame_number += 1;

        Ok(())
    }
}

impl Drop for WreRenderer {
    fn drop(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap() };

        for frame in &mut self.frames {
            unsafe {
                self.device.destroy_command_pool(frame.pool, None);

                //destroy sync objects
                self.device.destroy_fence(frame.render_fence, None);
                self.device
                    .destroy_semaphore(frame.swapchain_semaphore, None);
                self.device.destroy_semaphore(frame.render_semaphore, None);
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
