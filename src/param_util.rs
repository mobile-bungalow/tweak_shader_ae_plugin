use crate::types::ParamIdx;
use ae::aegp::DynamicStreamFlags;
use ae::ParamFlag;
use after_effects as ae;
use after_effects::aegp::suites;
use after_effects::{Error, InData};
use after_effects_sys::PF_Pixel;
use tweak_shader::input_type::InputType;

pub const MAX_INPUTS: i32 = 32;
pub const PARAM_TYPE_COUNT: i32 = 7;
pub const STATIC_PARAMS_OFFSET: i32 = ParamIdx::UseLayerTime.idx() + 1;
pub const PARAM_COUNT: i32 = (PARAM_TYPE_COUNT * MAX_INPUTS) + STATIC_PARAMS_OFFSET;

pub enum Variant {
    Float = 0,
    Int,
    IntList,
    Point,
    Bool,
    Color,
    Image,
}

pub fn index_from_mut(index: usize, variant: &mut tweak_shader::input_type::MutInput) -> ParamIdx {
    let variant = match variant.variant() {
        tweak_shader::input_type::InputVariant::Float => Variant::Float as _,
        tweak_shader::input_type::InputVariant::Int
            if variant.as_int().is_some_and(|e| e.labels.is_none()) =>
        {
            Variant::Int as _
        }
        tweak_shader::input_type::InputVariant::Int => Variant::IntList as _,
        tweak_shader::input_type::InputVariant::Point => Variant::Point as _,
        tweak_shader::input_type::InputVariant::Bool => Variant::Bool as _,
        tweak_shader::input_type::InputVariant::Color => Variant::Color as _,
        tweak_shader::input_type::InputVariant::Image => Variant::Image as _,
        _ => 0,
    };

    ParamIdx::Dynamic(((index as i32 * PARAM_TYPE_COUNT) + STATIC_PARAMS_OFFSET + variant) as u8)
}

pub fn as_param_index(index: usize, variant: &tweak_shader::input_type::InputType) -> ParamIdx {
    let variant = match variant {
        tweak_shader::input_type::InputType::Float(_) => Variant::Float as _,
        tweak_shader::input_type::InputType::Int(_, None) => Variant::Int as _,
        tweak_shader::input_type::InputType::Int(_, Some(_)) => Variant::IntList as _,
        tweak_shader::input_type::InputType::Point(_) => Variant::Point as _,
        tweak_shader::input_type::InputType::Bool(_) => Variant::Bool as _,
        tweak_shader::input_type::InputType::Color(_) => Variant::Color as _,
        tweak_shader::input_type::InputType::Image(_) => Variant::Image as _,
        _ => 0,
    };

    ParamIdx::Dynamic(((index as i32 * PARAM_TYPE_COUNT) + STATIC_PARAMS_OFFSET + variant) as u8)
}

pub fn update_param_defaults_and_labels(
    state: &mut crate::PluginState,
    local: &mut crate::Local,
) -> Result<(), ae::Error> {
    let Some(local_init) = local.local_init.as_mut() else {
        // Just show the load button if we haven't loaded
        // a shader.
        for i in ParamIdx::UnloadButton.idx()..PARAM_COUNT {
            set_param_visibility(state.in_data, ParamIdx::Dynamic(i as u8), false)?;
        }
        set_param_visibility(state.in_data, ParamIdx::LoadButton, true)?;

        return Ok(());
    };

    if !state
        .params
        .get(ParamIdx::UseLayerTime)?
        .as_checkbox()?
        .value()
    {
        set_param_visibility(state.in_data, ParamIdx::Time, true)?;
    }

    if !local_init.needs_param_visibility_reset() {
        return Ok(());
    }

    let param_util_suite = ae::pf::suites::ParamUtils::new()?;
    for (i, (name, var)) in local_init.ctx.iter_inputs().enumerate() {
        let index = as_param_index(i, var);
        set_param_visibility(state.in_data, index, true)?;
        let mut def = state.params.get_mut(index)?;
        def.set_name(name);
        let param = def.as_param_mut()?;
        match param {
            ae::Param::CheckBox(mut cb) => {
                if let InputType::Bool(b) = var {
                    cb.set_default(b.default.is_true());
                    cb.set_value(b.current.is_true());
                }
            }
            ae::Param::Color(mut co) => {
                if let InputType::Color(c) = var {
                    let val = c.default;
                    co.set_default(PF_Pixel {
                        alpha: (val[3] * 255.0) as u8,
                        red: (val[0] * 255.0) as u8,
                        green: (val[1] * 255.0) as u8,
                        blue: (val[2] * 255.0) as u8,
                    });

                    let val = c.current;
                    co.set_value(PF_Pixel {
                        alpha: (val[3] * 255.0) as u8,
                        red: (val[0] * 255.0) as u8,
                        green: (val[1] * 255.0) as u8,
                        blue: (val[2] * 255.0) as u8,
                    });
                }
            }
            ae::Param::FloatSlider(mut fl) => {
                if let InputType::Float(f) = var {
                    fl.set_default(f.default as f64);
                    fl.set_value(f.current as f64);
                    fl.set_valid_min(f.min);
                    fl.set_valid_max(f.max);
                    fl.set_slider_min(f.min);
                    fl.set_slider_max(f.max);
                }
            }
            ae::Param::Point(mut p) => {
                if let InputType::Point(pt) = var {
                    p.set_default(pt.default.into());
                    p.set_value(pt.current.into());
                }
            }
            ae::Param::Popup(mut il) => {
                if let InputType::Int(v, Some(_)) = var {
                    il.set_value(v.current);
                }
            }
            ae::Param::Slider(mut i) => {
                if let InputType::Int(v, None) = var {
                    i.set_default(v.default);
                    i.set_value(v.current);
                    i.set_valid_min(v.min);
                    i.set_valid_max(v.max);
                    i.set_slider_min(v.min);
                    i.set_slider_max(v.max);
                }
            }
            ae::Param::Layer(mut im) => {
                im.set_default_to_this_layer();
            }
            _ => {}
        }

        def.set_value_changed();
        param_util_suite.update_param_ui(state.in_data.effect(), index.idx(), &def)?;
    }

    local_init.finish_param_visibility_reset();
    Ok(())
}

pub fn update_param_ui(
    state: &mut crate::PluginState,
    local: &mut crate::Local,
) -> Result<(), ae::Error> {
    let Some(local_init) = local.local_init.as_mut() else {
        return Ok(());
    };

    for i in ParamIdx::UseLayerTime.idx()..PARAM_COUNT {
        set_param_visibility(state.in_data, ParamIdx::Dynamic(i as u8), false)?;
    }

    if local.src.is_none() || local_init.build_error.is_some() {
        set_param_visibility(state.in_data, ParamIdx::LoadButton, true)?;
        set_param_visibility(state.in_data, ParamIdx::Time, false)?;
        set_param_visibility(state.in_data, ParamIdx::UnloadButton, false)?;
        set_param_visibility(state.in_data, ParamIdx::ReloadButton, false)?;
        set_param_visibility(state.in_data, ParamIdx::IsImageFilter, false)?;
    } else {
        set_param_visibility(state.in_data, ParamIdx::LoadButton, false)?;
        set_param_visibility(state.in_data, ParamIdx::UnloadButton, true)?;
        set_param_visibility(state.in_data, ParamIdx::ReloadButton, true)?;
        set_param_visibility(state.in_data, ParamIdx::UseLayerTime, true)?;

        if !state
            .params
            .get(ParamIdx::UseLayerTime)?
            .as_checkbox()?
            .value()
        {
            set_param_visibility(state.in_data, ParamIdx::Time, true)?;
        } else {
            set_param_visibility(state.in_data, ParamIdx::Time, false)?;
        }

        for (i, (_, var)) in local_init.ctx.iter_inputs().enumerate() {
            let index = as_param_index(i, var);
            set_param_visibility(state.in_data, index, true)?;
        }

        let first_image_input = local_init
            .ctx
            .iter_inputs()
            .enumerate()
            .find(|(_, (_, ty))| ty.is_stored_as_texture());

        // only show image filter options IF we have at least one image input
        set_param_visibility(
            state.in_data,
            ParamIdx::IsImageFilter,
            first_image_input.is_some(),
        )?;

        // Toggle first image visibility if we are no longer a filter
        if let Some((i, (_, var))) = first_image_input {
            let index = as_param_index(i, var);

            let is_image_filter = state
                .params
                .get(ParamIdx::IsImageFilter)?
                .as_checkbox()?
                .value();

            set_param_visibility(state.in_data, index, !is_image_filter)?;
        }
    }

    Ok(())
}

fn default_flags() -> ParamFlag {
    ParamFlag::CANNOT_TIME_VARY
        | ParamFlag::TWIRLY
        | ParamFlag::SUPERVISE
        | ParamFlag::SKIP_REVEAL_WHEN_UNHIDDEN
}

// set up the params that every instance uses
pub fn setup_static_params(params: &mut ae::Parameters<ParamIdx>) -> Result<(), Error> {
    params.add_with_flags(
        ParamIdx::LoadButton,
        "Select Source",
        ae::ButtonDef::setup(|f| {
            f.set_label("Select Source");
        }),
        default_flags(),
        ae::ParamUIFlags::empty(),
    )?;

    params.add_with_flags(
        ParamIdx::UnloadButton,
        "Unload Source",
        ae::ButtonDef::setup(|f| {
            f.set_label("Unload Source");
        }),
        default_flags(),
        ae::ParamUIFlags::empty(),
    )?;

    params.add_with_flags(
        ParamIdx::ReloadButton,
        "Realod Source",
        ae::ButtonDef::setup(|f| {
            f.set_label("Resload Source");
        }),
        default_flags(),
        ae::ParamUIFlags::empty(),
    )?;

    params.add(ParamIdx::Time, "Time", ae::FloatSliderDef::setup(float))?;

    params.add_with_flags(
        ParamIdx::IsImageFilter,
        "Is Image Filter",
        ae::CheckBoxDef::setup(|f| {
            f.set_label("Enabled");
            f.set_default(true);
        }),
        default_flags(),
        ae::ParamUIFlags::empty(),
    )?;

    params.add_with_flags(
        ParamIdx::UseLayerTime,
        "Use Layer Time",
        ae::CheckBoxDef::setup(|f| {
            f.set_label("Enabled");
            f.set_default(true);
        }),
        default_flags(),
        ae::ParamUIFlags::empty(),
    )?;

    Ok(())
}

// create one param of every type to back
// a single input variant in the render context
pub fn create_variant_backing(params: &mut ae::Parameters<ParamIdx>) -> Result<(), Error> {
    let mut base_index = STATIC_PARAMS_OFFSET;
    for _ in 0..MAX_INPUTS {
        for offset in 0..PARAM_TYPE_COUNT {
            let name = format!("INPUT {}", base_index + offset);
            let index = ParamIdx::Dynamic(base_index as u8 + offset as u8);
            let ui_flags = ae::ParamUIFlags::empty();
            let param_flag = ParamFlag::TWIRLY | ParamFlag::SKIP_REVEAL_WHEN_UNHIDDEN;
            match offset as usize {
                f if f == Variant::Float as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::FloatSliderDef::setup(float),
                    param_flag,
                    ui_flags,
                )?,
                i if i == Variant::Int as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::SliderDef::setup(int),
                    param_flag,
                    ui_flags,
                )?,
                i if i == Variant::IntList as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::PopupDef::setup(options),
                    param_flag,
                    ui_flags,
                )?,
                pt if pt == Variant::Point as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::PointDef::setup(point),
                    param_flag,
                    ui_flags,
                )?,
                b if b == Variant::Bool as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::CheckBoxDef::setup(bool),
                    param_flag,
                    ui_flags,
                )?,
                c if c == Variant::Color as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::ColorDef::setup(color),
                    param_flag,
                    ui_flags,
                )?,
                i if i == Variant::Image as usize => params.add_with_flags(
                    index,
                    &name,
                    ae::LayerDef::setup(layer),
                    param_flag,
                    ui_flags,
                )?,
                _ => {}
            }
        }
        base_index += PARAM_TYPE_COUNT;
    }

    Ok(())
}

pub fn set_param_visibility(in_data: InData, index: ParamIdx, visible: bool) -> Result<(), Error> {
    let dyn_stream_suite = suites::DynamicStream::new()?;
    let stream_suite = suites::Stream::new()?;
    let interface = suites::PFInterface::new()?;

    // why unwrap or 10? it seems like if you don't register any AEGP hooks
    // your plugin ID is invalid, and using a plugin id that is *not* your assigned plugin ID
    // is the only way to make the aegp API work.
    let effect = interface
        .new_effect_for_effect(in_data.effect(), *crate::PLUGIN_ID.get().unwrap_or(&10))?;
    let stream = stream_suite.new_effect_stream_by_index(
        effect,
        *crate::PLUGIN_ID.get().unwrap_or(&10),
        index.idx(),
    )?;
    dyn_stream_suite.set_dynamic_stream_flag(
        stream,
        DynamicStreamFlags::Hidden,
        false,
        !visible,
    )?;

    Ok(())
}

fn layer(_f: &mut ae::LayerDef) {}

fn color(f: &mut ae::ColorDef) {
    f.set_default(ae::Pixel8 {
        alpha: 255,
        red: 255,
        blue: 255,
        green: 255,
    });
}

fn point(f: &mut ae::PointDef) {
    f.set_default((0.0, 0.0));
}

fn bool(f: &mut ae::CheckBoxDef) {
    f.set_label("Enabled");
    f.set_default(false);
}

fn options(f: &mut ae::PopupDef) {
    // it is unsafe to dynamically set options
    f.set_options(&["option 1", "option 2", "option 3", "option 4", "option 5"]);
    f.set_default(0);
}

fn int(f: &mut ae::SliderDef) {
    f.set_default(0);
    f.set_valid_min(-10_000);
    f.set_valid_max(10_000);
    f.set_slider_min(-100);
    f.set_slider_max(100);
}

fn float(f: &mut ae::FloatSliderDef) {
    f.set_default(0.);
    f.set_valid_min(-10_000.);
    f.set_valid_max(10_000.);
    f.set_slider_min(0.0);
    f.set_slider_max(1.0);
    f.set_precision(2);
}
