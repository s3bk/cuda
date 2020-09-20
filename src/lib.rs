#![feature(try_trait, ptr_internals)]

pub mod ffi;
use self::ffi::cuda::*;
use std::{mem, ptr, slice};
use std::os::raw::c_void;
use std::cmp::min;

pub mod device;
pub mod buffer;
//pub mod surface;
pub mod util;
//pub mod data;

pub use self::device::*;
pub use self::buffer::*;
pub use self::util::*;
//use self::data::*;

#[derive(Debug)]
pub enum CudaError {
    Other(cudaError_enum),
    Prohibited
}
impl From<cudaError_enum> for CudaError {
    fn from(v: cudaError_enum) -> CudaError {
        CudaError::Other(v)
    }
}


pub struct Context {
    handle: CUcontext
}
impl Context {
    pub fn create_module(&self, data: &mut String) -> Result<Module, CudaError> {
        unsafe {
            let mut module = mem::zeroed();
            
            let s = ZeroString::new(data);
            cuModuleLoadData(&mut module, s.ptr() as *const c_void)?;
            
            Ok(Module {
                module,
                context: self
            })
        }
    }
    pub fn create_module_null_terminated(&self, data: &[u8]) -> Result<Module, CudaError> {
        assert!(data.ends_with(b"\0"));
        unsafe {
            let mut module = mem::zeroed();
            
            cuModuleLoadData(&mut module, data.as_ptr() as *const c_void)?;
            
            Ok(Module {
                module,
                context: self
            })
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            cuCtxDestroy_v2(self.handle);
        }
    }
}

pub struct Module<'a> {
    module: CUmodule,
    context: &'a Context
}
impl<'a> Module<'a> {
    pub fn get(&self, name: &str) -> Result<Function, CudaError> {
        let mut name = String::from(name);
        
        let kernel = unsafe {
            let s = ZeroString::new(&mut name);
            let mut kernel = mem::zeroed();
            cuModuleGetFunction(&mut kernel, self.module, s.ptr())?;
            kernel
        };

        Ok(Function {
            func: kernel,
            name: name,
            module: self
        })
    }
}

pub struct Function<'a> {
    func: CUfunction,
    name: String,
    module: &'a Module<'a>
}
impl<'a> Function<'a> {
    /// this copies the given data into GPU memory
    /// and executes the kernel.
    /// The number and types of the parameters have to match those of the the function!
    #[inline]
    pub unsafe fn launch(&self, grid: [u32; 3], block: [u32; 3], shared_mem: u32, args: &mut [*mut c_void]) -> Result<(), CudaError>
    {
        println!("grid: {:?}, block: {:?}", grid, block);
        cuLaunchKernel(
            self.func,
            grid[0], grid[1], grid[2],
            block[0], block[1], block[2],
            shared_mem,
            ptr::null_mut(), // stream
            args.as_mut_ptr(),
            ptr::null_mut(), // parameters
        )?;
        cuCtxSynchronize()?;
        Ok(())
    }
    #[inline]
    pub unsafe fn launch_simple<T: Copy>(&self, data_in: &Buffer<T>, data_out: &mut Buffer<T>) -> Result<(), CudaError> {
        let batch = 512;
        let mut src = data_in.dev_ptr()?;
        let mut dst = data_out.dev_ptr()?;
        let mut args = [
            &mut src as *mut u64 as *mut c_void,
            &mut dst as *mut u64 as *mut c_void
        ];
        self.launch(
            [(data_in.len() / batch as usize) as u32, 1, 1],
            [batch, 1, 1],
            0,
            &mut args
        )?;
        data_out.set_len(data_in.len());
        Ok(())
    }
    /*
    pub unsafe fn launch_slice<'b, T, I, O>(&self, data_in: Slice<T, I>, mut data_out: Slice<T, O>) -> Result<(), CudaError>
        where T: Copy, I: Ref<'b>, O: Mut<'b>
    {
        let batch = 512;
        let (mut src, mut dst) = (0, 0);

        // get device pointers
        cuMemHostGetDevicePointer_v2(&mut src, data_in.as_ptr() as *mut c_void, 0)?;
        cuMemHostGetDevicePointer_v2(&mut dst, data_out.as_mut_ptr() as *mut c_void, 0)?;
        
        let mut args = [
            &mut src as *mut u64 as *mut c_void,
            &mut dst as *mut u64 as *mut c_void
        ];
        let len = min(data_in.len(), data_out.len());
        self.launch(
            [(len / batch as usize) as u32, 1, 1],
            [batch, 1, 1],
            0,
            &mut args
        )?;
        
        Ok(())
    }
    */
}
