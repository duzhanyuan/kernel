//
//  SOS: the Stupid Operating System
//  by Eliza Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2015-2017 Eliza Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
//! SOS init parameters
//!
//! This crate is intended to facilitate the sharing of initialization
//! parameters between "higher-level" SOS subcrates (such as [`alloc`] and
//! [`paging`]) in a platform-independent way.
//!
//! [`alloc`](../alloc)
//! [`paging`](../paging)
#![no_std]
#![deny(missing_docs)]
#![feature(step_trait)]

extern crate memory;
extern crate elf;
extern crate arrayvec;

use memory::{ PAddr, Page, PhysicalPage, FrameRange };
use core::default::Default;
use core::iter::Step;
use core::slice::Iter as SliceIter;
use arrayvec::{ArrayVec};

pub mod mem;

const MAX_MEM_AREAS: usize = 32;

/// If we are on x86_64 or armv7 this uses the 64-bit ELF word
#[cfg(target_pointer_width = "64")]
pub type ElfSections = elf::section::Sections<'static, u64>;

/// If we are on x86, this uses the 32-bit ELF word
#[cfg(target_pointer_width = "32")]
pub type ElfSections = elf::section::Sections<'static, u32>;

/// Parameters used during the init process
#[derive(Clone, Debug)]
pub struct InitParams {
    /// The base of the kernel memory range
    pub kernel_base: PAddr
  , /// The top of the kernel memory range
    pub kernel_top: PAddr
  , /// The base of the memory range for the kernel heap
    pub heap_base: PAddr
  , /// The top of the memory range to use for the kernel heap
    pub heap_top: PAddr
  , /// The base of the memory range for the kernel stack
    pub stack_base: PAddr
  , /// The top of the memory range to use for the kernel stack
    pub stack_top: PAddr
  , /// The start address of the Multiboot info structure, if it exists.
    ///
    /// N.B. that this is currently never `None`, as we only support multiboot.
    /// However, this may change at a later date.
    pub multiboot_start: Option<PAddr>
  , /// The end address of the Multiboot info structure, if it exists.
    ///
    /// N.B. that this is currently never `None`, as we only support multiboot.
    /// However, this may change at a later date.
    pub multiboot_end: Option<PAddr>
  , /// Map of memory areas
    pub mem_map: ArrayVec<[mem::Area; MAX_MEM_AREAS]>
    , /// Map of elf sections
    // todo: construct using convert::From<multiboot>
     pub elf_sections: Option<ElfSections>
}

impl Default for InitParams {
    fn default() -> Self {
        // use memory::arch::{HEAP_BASE, HEAP_TOP, STACK_BASE, STACK_TOP};
        InitParams { kernel_base: PAddr::from(0x0)
                     // NOTE: this is, of course, Extremely Wrong, but the
                     //       `Default` impl is not going to make _correct_
                     //       params, just fill in default values for other
                     //       fns that make params.
                     // TODO: should this be an Option instead?
                   , kernel_top: PAddr::from(0x0)
                   , heap_base:  PAddr::from(0x0)
                   , heap_top: PAddr::from(0x0)
                   , stack_base: PAddr::from(0x0)
                   , stack_top: PAddr::from(0x0)
                   , multiboot_start: None
                   , multiboot_end: None
                   , mem_map: ArrayVec::<[mem::Area; MAX_MEM_AREAS]>::new()
                   , elf_sections: None
                   }
    }
}

impl InitParams {
    /// Returns an iterator over the kernel's ELF sections
    // TODO: is this cross-platform? are we using ELF on all our supported
    //       architectures? i think we are, but we should ensure this is the
    //       case...
    //          – eliza, 1/22/2017
    pub fn elf_sections(&self) ->  ElfSections {
        self.elf_sections.clone()
        .expect("Attempted to access ELF sections on a \
                 non-ELF kernel!")
    }

    /// Returns the start address of the multiboot info struct
    ///
    /// # Panics
    /// If this is a non-Multiboot kernel
    // TODO: instead of panicking, return Option!
    //          - eliza, 5/26/2017
    #[inline]
    pub fn multiboot_start(&self) -> PAddr {
        self.multiboot_start
            .expect("Attempted to access Multiboot info structure on a \
                     non-Multiboot kernel!")
    }

    /// Returns the end address of the multiboot info struct
    ///
    /// # Panics
    /// If this is a non-Multiboot kernel
    // TODO: instead of panicking, return Option!
    //          - eliza, 5/26/2017
    pub fn multiboot_end(&self) -> PAddr {
        self.multiboot_end
            .expect("Attempted to access Multiboot info structure on a \
                     non-Multiboot kernel!")
    }

    /// Returns the range of frames containing the kernel binary.
    ///
    /// The kernel _should_ start on the first address in the frame range,
    /// since the kernel should be page aligned.
    //  TODO: this should be an array vector or linked list of frame ranges
    //        possibly?
    //          - eliza, 2/28/2017
    #[inline]
    pub fn kernel_frames(&self) -> FrameRange {
        // TODO: assert that the kernel base addr is page aligned here?
        //       this should maybe be a debug assertion?
        //          - eliza, 1/22/2017
        PhysicalPage::containing(self.kernel_base) ..
        PhysicalPage::containing(self.kernel_top).add_one()
    }

    /// Returns the range of frames containing the kernel heap
    ///
    /// The heap _should_ start on the first address in the frame range,
    /// since the heap should be page aligned.
    #[inline]
    pub fn heap_frames(&self) -> FrameRange {
        // TODO: assert that the heap base addr is page aligned here?
        //       this should maybe be a debug assertion?
        //          - eliza, 1/22/2017
        PhysicalPage::containing(self.heap_base) ..
        PhysicalPage::containing(self.heap_top).add_one()
    }

    /// Returns the range of frames containing the kernel stack.
    #[inline]
    pub fn stack_frames(&self) -> FrameRange {
        unimplemented!()
    }

    /// returns an iterator over the memory map
    #[inline]
    pub fn mem_map(&self) -> mem::Map {
        self.mem_map.iter()
    }


}
