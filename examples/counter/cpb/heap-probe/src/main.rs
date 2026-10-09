//! Host-side heap probe for the counter's core, built without std as the
//! firmware builds it. Drives it the way the cpb shell does and reports peak
//! heap use. Host pointers are 8 bytes against 4 on the nRF52840, so treat the
//! result as an upper bound.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use shared::{Core, Counter, Effect, Event};

struct Tracking;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(now, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Tracking = Tracking;

fn main() {
    let base = CURRENT.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);

    let core: Core<Counter> = Core::new();
    let mut renders = 0;

    // The shell's handler: every render reads the view, as the firmware does.
    let mut handle = |effects: Vec<Effect>| {
        for effect in effects {
            match effect {
                Effect::Render(_) => {
                    let _ = core.view();
                    renders += 1;
                }
            }
        }
    };

    // The firmware draws the initial view without an event.
    let _ = core.view();

    // Bursts of presses, as fast as the shell can take them. Each event
    // finishes with its render, so nothing is left in flight between presses.
    for burst in [1usize, 5, 20, 20, 20, 1, 1] {
        for i in 0..burst {
            let event = if i % 2 == 0 {
                Event::Increment
            } else {
                Event::Decrement
            };
            handle(core.process_event(event));
        }
        println!(
            "burst of {burst:>2} presses: peak heap so far {} B, live after {} B",
            PEAK.load(Ordering::Relaxed) - base,
            CURRENT.load(Ordering::Relaxed) - base
        );
    }
    println!("renders: {renders}, value: {}", core.view().value);
}
