use after_effects as ae;
use tweak_shader::wgpu::{self, Device, Queue};

// converts wgpu texture buffers from u15 to 32 float, and from float back to u16.
// preprocessing handles converting to the right color space and swizzling.
#[derive(Debug)]
pub struct U16ConversionContext {
    u16_to_fp_ctx: tweak_shader::RenderContext,
    fp_to_u16_ctx: tweak_shader::RenderContext,
    // input layer textures are rendered into FP, with properly sized
    // buffers here to avoid thrashing vram.
    fp_staging_textures: std::collections::HashMap<String, wgpu::Texture>,
    // the main shader renders to this, this is converted
    // by fp_to_u16 to ae format, written into a buffer
    fp16_output_texture: Option<wgpu::Texture>,
}

impl U16ConversionContext {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            u16_to_fp_ctx: tweak_shader::RenderContext::new(
                include_str!("./resources/to_fp.fs"),
                wgpu::TextureFormat::Rgba16Float,
                device,
                queue,
            )
            .unwrap(),
            fp_to_u16_ctx: tweak_shader::RenderContext::new(
                include_str!("./resources/to_u15.fs"),
                wgpu::TextureFormat::Rgba16Uint,
                device,
                queue,
            )
            .unwrap(),
            fp_staging_textures: Default::default(),
            fp16_output_texture: None,
        }
    }

    // A note to people looking at this:
    // the tweak shader library will
    // convert all unorm shaders to output ARGB format
    // when compiled with the "after_effects" features.
    pub fn render_u15_to_cpu_buffer(
        &mut self,
        out_layer: &mut ae::Layer,
        device: &Device,
        queue: &Queue,
        main_render_ctx: &mut tweak_shader::RenderContext,
    ) {
        for (name, tex) in self.fp_staging_textures.iter() {
            main_render_ctx.load_shared_texture(tex, name);
        }

        let width = out_layer.width() as u32;
        let height = out_layer.height() as u32;

        let target_texture = if self
            .fp16_output_texture
            .as_ref()
            .is_some_and(|t| t.width() == width && t.height() == height)
        {
            self.fp16_output_texture.as_ref().unwrap()
        } else {
            let new_tex = device.create_texture(&target_desc(
                width,
                height,
                wgpu::TextureFormat::Rgba16Float,
            ));
            self.fp16_output_texture = Some(new_tex);
            self.fp16_output_texture.as_ref().unwrap()
        };

        main_render_ctx.render(
            queue,
            device,
            &target_texture.create_view(&Default::default()),
            width,
            height,
        );

        // Update resolutions
        self.fp_to_u16_ctx
            .get_input_mut("height")
            .unwrap()
            .as_float()
            .unwrap()
            .current = height as f32;

        self.fp_to_u16_ctx
            .get_input_mut("width")
            .unwrap()
            .as_float()
            .unwrap()
            .current = width as f32;

        self.fp_to_u16_ctx
            .load_shared_texture(target_texture, "input_image");

        self.fp_to_u16_ctx
            .render_to_slice(queue, device, width, height, out_layer.buffer_mut());
    }

    // Loads or creates all textures from the iterator into staging buffers.
    // converts from u15 to floating point 32
    pub fn prepare_cpu_layer_inputs<'a, I>(&mut self, device: &Device, queue: &Queue, layers: I)
    where
        I: Iterator<Item = (&'a str, ae::Layer)>,
    {
        let mut render_encoder = device.create_command_encoder(&Default::default());
        for (name, layer) in layers {
            let width = layer.width() as u32;
            let height = layer.height() as u32;

            let texture = if self
                .fp_staging_textures
                .get(name)
                .is_some_and(|t| t.width() == width && t.height() == height)
            {
                self.fp_staging_textures.get(name).unwrap()
            } else {
                let new_tex = device.create_texture(&target_desc(
                    width,
                    height,
                    wgpu::TextureFormat::Rgba16Float,
                ));
                self.fp_staging_textures.insert(name.to_string(), new_tex);
                self.fp_staging_textures.get(name).unwrap()
            };

            self.u16_to_fp_ctx.load_image_immediate(
                "input_image",
                height,
                width,
                layer.row_bytes() as u32,
                device,
                queue,
                &wgpu::TextureFormat::Rgba16Unorm,
                layer.buffer(),
            );

            self.u16_to_fp_ctx
                .get_input_mut("height")
                .unwrap()
                .as_float()
                .unwrap()
                .current = height as f32;

            self.u16_to_fp_ctx
                .get_input_mut("width")
                .unwrap()
                .as_float()
                .unwrap()
                .current = width as f32;

            self.u16_to_fp_ctx.encode_render(
                queue,
                device,
                &mut render_encoder,
                &texture.create_view(&Default::default()),
                width,
                height,
            );
        }
        queue.submit([render_encoder.finish()]);
    }
}

fn target_desc(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1, // crunch crunch
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }
}
