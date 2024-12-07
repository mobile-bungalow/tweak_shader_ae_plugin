use crate::{preprocessing, u15_conversion::*};
use after_effects::PixelFormat;
use serde::{Deserialize, Serialize};
use tweak_shader::wgpu::{self, Device, Queue};

#[repr(u8)]
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub enum ParamIdx {
    LoadButton = 1,
    UnloadButton = 2,
    Time = 3,
    IsImageFilter = 4,
    UseLayerTime = 5,
    Dynamic(u8),
}

impl std::cmp::Eq for ParamIdx {}

impl ParamIdx {
    pub const fn idx(&self) -> i32 {
        match self {
            ParamIdx::LoadButton => 1,
            ParamIdx::UnloadButton => 2,
            ParamIdx::Time => 3,
            ParamIdx::IsImageFilter => 4,
            ParamIdx::UseLayerTime => 5,
            ParamIdx::Dynamic(x) => *x as i32,
        }
    }
}

impl From<u8> for ParamIdx {
    fn from(value: u8) -> Self {
        match value {
            1 => ParamIdx::LoadButton,
            2 => ParamIdx::UnloadButton,
            3 => ParamIdx::Time,
            4 => ParamIdx::IsImageFilter,
            5 => ParamIdx::UseLayerTime,
            _ => ParamIdx::Dynamic(value),
        }
    }
}

impl From<ParamIdx> for u8 {
    fn from(value: ParamIdx) -> Self {
        match value {
            ParamIdx::LoadButton => 1,
            ParamIdx::UnloadButton => 2,
            ParamIdx::Time => 3,
            ParamIdx::IsImageFilter => 4,
            ParamIdx::UseLayerTime => 5,
            ParamIdx::Dynamic(x) => x,
        }
    }
}

#[derive(Debug)]
pub enum TweakError {
    SetUp(String),
}

impl std::fmt::Display for TweakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetUp(s) => f.write_str(s),
        }
    }
}

impl From<TweakError> for super::Error {
    fn from(value: TweakError) -> Self {
        match value {
            TweakError::SetUp(_) => Self::Generic,
        }
    }
}

impl std::error::Error for TweakError {}

#[derive(Debug, Copy, Clone)]
#[repr(i16)]
pub enum BitDepth {
    U8 = 8,
    U16 = 16,
    F32 = 32,
    Invalid(i16),
}

impl From<i16> for BitDepth {
    fn from(value: i16) -> Self {
        match value {
            8 => BitDepth::U8,
            16 => BitDepth::U16,
            32 => BitDepth::F32,
            v => BitDepth::Invalid(v),
        }
    }
}

#[derive(Debug)]
pub enum PixelFormatConversionError {
    InvalidFormat,
}

pub fn try_into(value: PixelFormat) -> Result<wgpu::TextureFormat, PixelFormatConversionError> {
    match value {
        after_effects::PixelFormat::Argb32 => Ok(wgpu::TextureFormat::Rgba8Unorm),
        after_effects::PixelFormat::Argb64 => Ok(wgpu::TextureFormat::Rgba16Float),
        after_effects::PixelFormat::Argb128 => Ok(wgpu::TextureFormat::Rgba32Float),
        after_effects::PixelFormat::Bgra32 => Ok(wgpu::TextureFormat::Bgra8Unorm),
        // Invalid format
        _ => Err(PixelFormatConversionError::InvalidFormat),
    }
}

impl TryFrom<BitDepth> for wgpu::TextureFormat {
    type Error = i16;
    fn try_from(value: BitDepth) -> Result<wgpu::TextureFormat, Self::Error> {
        match value {
            BitDepth::U8 => Ok(wgpu::TextureFormat::Rgba8Unorm),
            BitDepth::U16 => Ok(wgpu::TextureFormat::Rgba16Float),
            BitDepth::F32 => Ok(wgpu::TextureFormat::Rgba32Float),
            BitDepth::Invalid(v) => Err(v),
        }
    }
}

impl From<wgpu::TextureFormat> for BitDepth {
    fn from(value: wgpu::TextureFormat) -> Self {
        match value {
            wgpu::TextureFormat::Rgba8Unorm => BitDepth::U8,
            wgpu::TextureFormat::Rgba16Float => BitDepth::U16,
            wgpu::TextureFormat::Rgba32Float => BitDepth::F32,
            _ => BitDepth::Invalid(-42), // You may want to handle other cases as needed
        }
    }
}

pub enum TweakShaderGlobal {
    Init(InnerGlobal),
    Uninit,
}

impl TweakShaderGlobal {
    pub fn as_init(&self) -> Option<&InnerGlobal> {
        match self {
            Self::Init(a) => Some(a),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct InnerGlobal {
    pub device: Device,
    pub queue: Queue,
}

after_effects::define_cross_thread_type!(Local);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Local {
    // Post initialization only fields
    #[serde(skip_serializing, skip_deserializing)]
    pub local_init: Option<LocalInit>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub src: Option<String>,
}

#[derive(Debug)]
pub struct LocalInit {
    pub ctx: tweak_shader::RenderContext,
    needs_param_setup: bool,
    pub fmt: wgpu::TextureFormat,
    pub build_error: Option<String>,
    pub u16_converter: Option<U16ConversionContext>,
}

impl Default for TweakShaderGlobal {
    fn default() -> Self {
        // GPU buffers on windows are in Cuda.
        let instance_desc = wgpu::InstanceDescriptor {
            #[cfg(target_os = "windows")]
            backends: wgpu::Backends::VULKAN,
            #[cfg(target_os = "macos")]
            backends: wgpu::Backends::METAL,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(instance_desc);

        let maybe_adapter = pollster::block_on(async {
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
        });

        let Some(adapter) = maybe_adapter else {
            return Self::Uninit;
        };

        let mut required_limits =
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());

        required_limits.max_push_constant_size = 256;
        required_limits.max_storage_textures_per_shader_stage = 4;

        let maybe_dq = pollster::block_on(async {
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::PUSH_CONSTANTS
                            | wgpu::Features::TEXTURE_FORMAT_16BIT_NORM
                            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                            | wgpu::Features::VERTEX_WRITABLE_STORAGE,
                        required_limits,
                    },
                    None,
                )
                .await
        });

        let (device, queue) = match maybe_dq {
            Err(_) => return Self::Uninit,
            Ok((device, queue)) => (device, queue),
        };

        device.on_uncaptured_error(Box::new(|e| match e {
            wgpu::Error::Internal {
                source,
                description,
            } => {
                panic!("Internal GPU Error! {source} : {description}");
            }
            wgpu::Error::OutOfMemory { .. } => {
                panic!("Out of memory");
            }
            wgpu::Error::Validation {
                description,
                source,
            } => {
                panic!("{description} : {source}");
            }
        }));

        TweakShaderGlobal::Init(InnerGlobal { device, queue })
    }
}

impl Drop for TweakShaderGlobal {
    fn drop(&mut self) {
        CrossThreadLocal::clear_map();
    }
}

impl LocalInit {
    fn new(device: &Device, queue: &Queue, fmt: wgpu::TextureFormat, src: Option<String>) -> Self {
        let mut build_error = None;

        let ctx = src
            .ok_or("No Source in initialization".to_owned())
            .and_then(|src| preprocessing::convert_output_to_ae_format(&src))
            .and_then(|src| {
                tweak_shader::RenderContext::new(src, fmt, device, queue)
                    .map_err(|e| format!("{e}"))
            });

        let ctx = match ctx {
            Ok(okay) => okay,
            Err(e) => {
                let error_shader = preprocessing::convert_output_to_ae_format(include_str!(
                    "./resources/error.fs"
                ))
                .unwrap();

                build_error = Some(e.to_string());
                tweak_shader::RenderContext::new(&error_shader, fmt, device, queue).unwrap()
            }
        };

        let u16_converter = if fmt == wgpu::TextureFormat::Rgba16Float {
            Some(U16ConversionContext::new(device, queue))
        } else {
            None
        };

        LocalInit {
            ctx,
            fmt,
            needs_param_setup: true,
            build_error,
            u16_converter,
        }
    }

    pub fn queue_param_visibility_reset(&mut self) {
        self.needs_param_setup = true
    }

    pub fn needs_param_visibility_reset(&self) -> bool {
        self.needs_param_setup
    }

    pub fn finish_param_visibility_reset(&mut self) {
        self.needs_param_setup = false;
    }
}

impl Local {
    pub fn init_or_update(&mut self, device: &Device, queue: &Queue, bit_depth: BitDepth) {
        match self.local_init {
            None => {
                let expected_fmt: wgpu::TextureFormat = bit_depth
                    .try_into()
                    .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
                self.local_init = Some(LocalInit::new(
                    device,
                    queue,
                    expected_fmt,
                    self.src.clone(),
                ));
            }
            Some(LocalInit { fmt, .. }) => {
                if let Ok(expected_fmt) = bit_depth.try_into() {
                    if fmt != expected_fmt {
                        self.local_init = Some(LocalInit::new(
                            device,
                            queue,
                            expected_fmt,
                            self.src.clone(),
                        ));
                    }
                }
            }
        }
    }

    pub fn launch_shader_selection_dialog(&mut self, global: &TweakShaderGlobal) -> Option<String> {
        let InnerGlobal { queue, device, .. } = global.as_init()?;

        let home_dir = match homedir::get_my_home() {
            Ok(Some(home)) => home,
            _ => "/".into(),
        };

        let file = rfd::FileDialog::new()
            .add_filter("shader", &["glsl", "fs", "vs", "frag"])
            .set_directory(home_dir)
            .pick_file();

        let source = file.map(|path| std::fs::read_to_string(path).unwrap_or_default());
        let mut local_init = LocalInit::new(
            device,
            queue,
            self.local_init
                .as_ref()
                .map(|l| l.fmt)
                .unwrap_or(wgpu::TextureFormat::Rgba8Unorm),
            source.clone(),
        );
        local_init.needs_param_setup = true;
        let out = local_init.build_error.clone();

        self.src = source;
        self.local_init = Some(local_init);
        out
    }

    pub fn unload_scene(&mut self) {
        self.src = None;
        self.local_init = None;
    }
}
