use tweak_shader::TextureDesc;

use super::*;

// Runs the user shader, copying the results from the GPU to RAM.
pub fn render(
    state: &mut super::PluginState,
    instance: &mut super::Local,
    extra: &SmartRenderExtra,
) -> Result<(), after_effects::Error> {
    let Some(global) = state.global.as_init() else {
        return Err(Error::Generic);
    };

    let local = instance.local_init.as_mut();
    let Some(LocalInit {
        ref mut ctx,
        u16_converter,
        fmt,
        ..
    }) = local
    else {
        return Err(Error::Generic);
    };
    let layers = load_parameters(ctx, state)?;

    let cb = extra.callbacks();

    let layer_iter = layers.iter().filter_map(|(name, index)| {
        Some((
            name.as_str(),
            cb.checkout_layer_pixels(index.idx() as u32).ok()??,
        ))
    });

    if let Some(converter) = u16_converter {
        converter.prepare_cpu_layer_inputs(&global.device, &global.queue, layer_iter);

        let Some(mut out_layer) = cb.checkout_output()? else {
            return Ok(());
        };

        ctx.update_resolution([out_layer.width() as f32, out_layer.height() as f32]);
        converter.render_u15_to_cpu_buffer(&mut out_layer, &global.device, &global.queue, ctx);
    } else {
        for (name, layer) in layer_iter {
            let real_fmt = layer.pixel_format().map(types::try_into)?.unwrap_or(*fmt);
            ctx.load_texture(
                name,
                TextureDesc {
                    width: layer.width() as u32,
                    height: layer.height() as u32,
                    stride: Some(layer.buffer_stride() as u32),
                    data: layer.buffer(),
                    format: real_fmt,
                },
                &global.device,
                &global.queue,
            );
        }

        let Some(mut out_layer) = cb.checkout_output()? else {
            return Ok(());
        };

        let stride = out_layer.buffer_stride();

        ctx.update_resolution([out_layer.width() as f32, out_layer.height() as f32]);
        ctx.render_to_slice(
            &global.queue,
            &global.device,
            out_layer.width() as u32,
            out_layer.height() as u32,
            out_layer.buffer_mut(),
            Some(stride as u32),
        );
    }

    Ok(())
}

// Load params from AE to tweak shader
pub fn load_parameters(
    ctx: &mut tweak_shader::RenderContext,
    state: &super::PluginState,
) -> Result<Vec<(String, ParamIdx)>, after_effects::Error> {
    let in_data = state.in_data;
    let current_time = in_data.current_time();
    let current_frame = state.in_data.current_frame();
    let current_delta = in_data.time_step();
    let time_step = in_data.time_step();
    let time_scale = in_data.time_scale();
    let mut non_null_images = Vec::new();
    let mut null_images = Vec::new();

    let is_image_filter = state
        .params
        .get(ParamIdx::IsImageFilter)?
        .as_checkbox()?
        .value();

    let mut first_image = true;

    for (i, (name, mut input)) in ctx.iter_inputs_mut().enumerate() {
        let index = param_util::index_from_mut(i, &mut input);

        let mut param = ParamDef::checkout(
            in_data,
            index.idx(),
            current_time,
            time_step,
            time_scale,
            None,
        )?;

        match param.as_param_mut()? {
            Param::CheckBox(cb) => {
                if let Some(boolean) = input.as_bool() {
                    boolean.current = if cb.value() {
                        tweak_shader::input_type::ShaderBool::True
                    } else {
                        tweak_shader::input_type::ShaderBool::False
                    };
                }
            }
            Param::Color(co) => {
                if let Some(color) = input.as_color() {
                    let val = co.value();
                    color.current = [
                        val.red as f32 / 255.0,
                        val.green as f32 / 255.0,
                        val.blue as f32 / 255.0,
                        val.alpha as f32 / 255.0,
                    ];
                }
            }
            Param::FloatSlider(fl) => {
                if let Some(float) = input.as_float() {
                    float.current = fl.value() as f32;
                }
            }
            Param::Slider(int) => {
                if let Some(ount) = input.as_int() {
                    ount.value.current = int.value();
                }
            }
            Param::Point(pt) => {
                if let Some(point) = input.as_point() {
                    point.current = pt.value().into();
                }
            }
            Param::Popup(int) => {
                if let Some(ount) = input.as_int() {
                    if let Some(entry) = ount
                        .labels
                        .as_ref()
                        .and_then(|l| l.get(int.value() as usize - 1))
                        .map(|(_, v)| v)
                    {
                        ount.value.current = *entry;
                    }
                }
            }
            Param::Layer(l) => {
                if first_image && is_image_filter {
                    first_image = false;
                    non_null_images.push((name.to_owned(), INPUT_LAYER_CHECKOUT_ID));
                    continue;
                }
                if l.value().is_some() {
                    non_null_images.push((name.to_owned(), index));
                } else {
                    null_images.push(name.to_owned());
                }
            }
            _ => {}
        }
    }

    for image_name in null_images {
        ctx.remove_texture(&image_name);
    }

    let use_layer_time = state
        .params
        .get(ParamIdx::UseLayerTime)?
        .as_checkbox()?
        .value();

    if !use_layer_time {
        let time = state.params.get(ParamIdx::Time)?.as_float_slider()?.value();
        ctx.update_time(time as f32);
    } else {
        ctx.update_time(current_time as f32 / time_scale as f32);
    }

    ctx.update_frame_count(current_frame as u32);
    ctx.update_delta(current_delta as f32);

    Ok(non_null_images)
}
