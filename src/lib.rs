mod param_util;
mod preprocessing;
mod render;
mod types;
mod window_handle;

mod u15_conversion;
use std::sync::Mutex;

use ae::*;
use after_effects as ae;
use after_effects_sys as ae_sys;
use types::*;

const SERDE_ID: u16 = 1;
const INPUT_LAYER_CHECKOUT_ID: ParamIdx = ParamIdx::Dynamic(240);
static PLUGIN_ID: std::sync::OnceLock<i32> = std::sync::OnceLock::new();

ae::define_effect!(TweakShaderGlobal, LocalMutex, ParamIdx);

macro_rules! lock {
    ( $mutex_arc:expr ) => {
        $mutex_arc.lock().unwrap()
    };
}

impl AdobePluginInstance for LocalMutex {
    fn flatten(&self) -> Result<(u16, Vec<u8>), Error> {
        let out = bincode::serialize(&lock!(self).src).map_err(|_| Error::Generic)?;
        Ok((SERDE_ID, out))
    }

    fn unflatten(version: u16, serialized: &[u8]) -> Result<Self, Error> {
        match version {
            SERDE_ID => {
                let src: Option<String> =
                    bincode::deserialize(serialized).map_err(|_| Error::Generic)?;
                let mut out = Local::default();
                out.local_init = None;
                out.src = src;
                Ok(Mutex::new(out))
            }
            _ => Err(Error::Generic),
        }
    }

    fn render(&self, _: &mut PluginState, _: &Layer, _: &mut Layer) -> Result<(), ae::Error> {
        // We smart render,
        Ok(())
    }

    fn do_dialog(&mut self, _: &mut PluginState) -> Result<(), ae::Error> {
        Ok(())
    }

    fn handle_command(&mut self, plugin: &mut PluginState, command: Command) -> Result<(), Error> {
        let PluginState {
            out_data, in_data, ..
        } = plugin;
        match command {
            Command::About => {
                out_data.set_return_msg("Tweak Shader, v2.0, The flexible shader plugin.")
            }
            Command::UpdateParamsUi => {
                param_util::update_param_defaults_and_labels(plugin, &mut lock!(self))?;
                param_util::update_param_ui(plugin, &mut lock!(self))?;
            }
            Command::UserChangedParam { param_index } => {
                match ParamIdx::from(param_index as u8) {
                    ParamIdx::UnloadButton => {
                        lock!(self).unload_scene();
                        param_util::update_param_defaults_and_labels(plugin, &mut lock!(self))?;
                    }
                    ParamIdx::LoadButton => {
                        let error_message =
                            lock!(self).launch_shader_selection_dialog(plugin.global);
                        if let Some(err) = error_message {
                            out_data.set_error_msg(&err);
                        } else {
                            param_util::update_param_defaults_and_labels(plugin, &mut lock!(self))?;
                        }
                    }
                    ParamIdx::IsImageFilter => {
                        if let Some(init) = lock!(self).local_init.as_mut() {
                            init.queue_param_visibility_reset();
                        }

                        let is_image_filter = plugin
                            .params
                            .get(ParamIdx::IsImageFilter)?
                            .as_checkbox()?
                            .value();

                        let first_image = lock!(self)
                            .local_init
                            .as_ref()
                            .and_then(|init| {
                                init.ctx
                                    .iter_inputs()
                                    .enumerate()
                                    .find(|(_, (_, i))| i.is_stored_as_texture())
                                    .map(|(i, (_, ty))| param_util::as_param_index(i, ty))
                            })
                            .clone();

                        if let Some(index) = first_image {
                            if is_image_filter {
                                let mut param = plugin.params.get_mut(index)?;
                                let mut layer = param.as_layer_mut()?;
                                layer.set_default_to_this_layer();
                            }

                            param_util::set_param_visibility(
                                plugin.in_data,
                                index,
                                !is_image_filter,
                            )?;
                        }
                    }
                    _ => {}
                }
                plugin.out_data.set_force_rerender();
            }
            Command::SmartPreRender { mut extra } => {
                let mut req = extra.output_request();

                let cb = extra.callbacks();

                if let Some(global) = plugin.global.as_init() {
                    lock!(self).init_or_update(
                        &global.device,
                        &global.queue,
                        extra.bit_depth().into(),
                    );

                    let current_time = in_data.current_time();
                    let time_step = in_data.time_step();
                    let time_scale = in_data.time_scale();

                    if let Some(LocalInit { ctx, .. }) = lock!(self).local_init.as_ref() {
                        for (index, (_, v)) in ctx
                            .iter_inputs()
                            .enumerate()
                            .filter(|(_, (_, v))| v.is_stored_as_texture())
                        {
                            let id_and_index = param_util::as_param_index(index, v).idx();

                            cb.checkout_layer(
                                id_and_index,
                                id_and_index,
                                &req,
                                current_time,
                                time_step,
                                time_scale,
                            )?;
                        }
                    }
                }

                req.field = ae_sys::PF_Field_FRAME as i32;
                req.preserve_rgb_of_zero_alpha = 1;
                req.channel_mask = ae_sys::PF_ChannelMask_ARGB as i32;

                // We checkout once just to see what the max rect is :(
                if let Ok(width_test) = cb.checkout_layer(
                    0,
                    INPUT_LAYER_CHECKOUT_ID.idx() - 1,
                    &req,
                    in_data.current_time(),
                    in_data.time_step(),
                    in_data.time_scale(),
                ) {
                    req.rect = width_test.max_result_rect;

                    let full_checkout = cb.checkout_layer(
                        0,
                        INPUT_LAYER_CHECKOUT_ID.idx(),
                        &req,
                        in_data.current_time(),
                        in_data.time_step(),
                        in_data.time_scale(),
                    )?;

                    extra.set_result_rect(full_checkout.result_rect.into());
                    extra.set_max_result_rect(full_checkout.result_rect.into());
                    extra.set_returns_extra_pixels(true);
                }
            }
            Command::SmartRender { extra } => {
                render::render(plugin, &mut lock!(self), &extra)?;
            }
            Command::SequenceSetup => {
                if let Some(global) = plugin.global.as_init() {
                    lock!(self).init_or_update(&global.device, &global.queue, BitDepth::U8);
                }
            }
            Command::SequenceResetup => {
                if let Some(global) = plugin.global.as_init() {
                    lock!(self).init_or_update(&global.device, &global.queue, BitDepth::U8);
                }
            }
            _ => {}
        };

        Ok(())
    }
}

impl AdobePluginGlobal for TweakShaderGlobal {
    fn can_load(_host_name: &str, _host_version: &str) -> bool {
        true
    }

    fn params_setup(
        &self,
        params: &mut ae::Parameters<ParamIdx>,
        _in_data: InData,
        _out_data: OutData,
    ) -> Result<(), Error> {
        param_util::setup_static_params(params)?;
        param_util::create_variant_backing(params)?;
        Ok(())
    }

    fn handle_command(
        &mut self,
        cmd: ae::Command,
        _in_data: ae::InData,
        mut out_data: ae::OutData,
        _params: &mut ae::Parameters<ParamIdx>,
    ) -> Result<(), ae::Error> {
        match cmd {
            ae::Command::About => {
                out_data.set_return_msg("The Tweak shader flexible shader plugin.");
            }
            Command::GlobalSetup => {
                let suite = ae::aegp::suites::Utility::new()?;

                PLUGIN_ID
                    .set(suite.register_with_aegp(None, "tweak_shader")?)
                    .expect("already set");

                if let TweakShaderGlobal::Uninit = self {
                    out_data.set_return_msg("Tweak Shader Failed to initialize");
                    return Err(ae::Error::Generic);
                };
            }
            _ => {}
        }
        Ok(())
    }
}
