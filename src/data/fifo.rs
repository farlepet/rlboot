extern crate alloc;

use core::{
    alloc::Layout, cell::UnsafeCell, fmt::Error, mem, ptr::NonNull, sync::atomic::{AtomicUsize, Ordering},
};

/// FIFO buffer that can be written and read from simultaneously. Writes require
/// a mutable reference, while reads do not.
pub struct FIFO<T: Copy> {
    data: NonNull<T>,              //< FIFO data
    size: usize,                   //< Maximum number of elements that can be held in the FIFO
    head: UnsafeCell<AtomicUsize>, //< Oldest entry in FIFO
    tail: AtomicUsize,             //< Location for next entry placed in FIFO
}

impl<T: Copy> FIFO<T> {
    pub fn new(size: usize) -> FIFO<T> {
        let layout = Layout::from_size_align(size * mem::size_of::<T>(), mem::align_of::<T>()).unwrap();
        let buffer = unsafe {
            alloc::alloc::alloc(layout)
        };
        FIFO {
            data: NonNull::new(buffer as *mut T).unwrap(),
            size,
            head: UnsafeCell::new(AtomicUsize::new(0)),
            tail: AtomicUsize::new(0),
        }
    }

    /// Add new item to the FIFO. If FIFO is full, data is dropped and an error
    /// is returned.
    pub fn enqueue(&mut self, value: T) -> Result<(), Error> {
        if self.free() == 0 {
            return Err(Error);
        }

        let tail_val = self.tail.load(Ordering::Relaxed);
        unsafe {
            *self.data.as_ptr().add(tail_val) = value;
        }
        self.tail.store(if tail_val == (self.size) - 1 { 0 } else { tail_val + 1 },
                        Ordering::Relaxed);

        Ok(())
    }

    /// Remove oldest item from the FIFO
    pub fn dequeue(&self) -> Option<T> {
        if self.len() == 0 {
            return None;
        }

        let head = self.head.get();
        let head_val = unsafe { (*head).load(Ordering::Relaxed) };
        let item = unsafe {
            *self.data.as_ptr().add(head_val)
        };

        unsafe {
            (*head).store(if head_val == (self.size - 1) { 0 } else { head_val + 1 },
                          Ordering::Relaxed);
        }

        Some(item)
    }

    /// Get current number of elements in the FIFO
    /// Not thread/interrupt safe.
    pub fn len(&self) -> usize {
        let head = unsafe {
            (*self.head.get()).load(Ordering::Relaxed)
        };
        let tail = self.tail.load(Ordering::Relaxed);

        if head <= tail {
            tail - head
        } else {
            (head - tail) - 1
        }
    }

    /// Get current number of free spaces for elements in the FIFO
    /// Not thread/interrupt safe.
    pub fn free(&self) -> usize {
        let head = unsafe {
            (*self.head.get()).load(Ordering::Relaxed)
        };
        let tail = self.tail.load(Ordering::Relaxed);

        if head <= tail {
            (self.size - (tail - head)) - 1
        } else {
            self.size - (head - tail)
        }
    }
}

