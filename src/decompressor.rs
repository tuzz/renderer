use std::{thread, collections::BinaryHeap, cmp, ops};
use std::sync::{Arc, atomic::AtomicUsize};
use crossbeam_channel::{Sender, Receiver};

pub struct Decompressor;

struct Worker {
    pub thread: thread::JoinHandle<()>,
    pub receiver: Receiver<crate::StreamFrame>,
}

impl Decompressor {
    pub fn new(_directory: &str, _concurrent: bool, _remove_after: bool) -> Self {
        Self
    }

    pub fn decompress_from_disk(&self, mut process_function: Box<dyn FnMut(crate::StreamFrame)>) {
        // TODO: run for each timestamped group of files (one thread per file)

        let mut workers: Vec<Worker> = vec![];
        order_frames_from_worker_threads(workers, &mut process_function);
    }
}

fn order_frames_from_worker_threads(mut workers: Vec<Worker>, process_function: &mut Box<dyn FnMut(crate::StreamFrame)>) {
    let mut min_heap = BinaryHeap::new();
    let mut expected_frame = 0;

    loop {
        // Ask each worker for their next stream frame. Remove workers that have finished.
        let drained = workers.drain_filter(|worker| {
            if let Ok(stream_frame) = worker.receiver.recv() {
                min_heap.push(cmp::Reverse(OrderableFrame(stream_frame)));
                false
            } else {
                true
            }
        });

        // Panic if a worker didn't terminate properly.
        for worker in drained { worker.thread.join().unwrap(); }

        let mut found_a_frame = false;

        // Keep getting the next ordered frame from the heap until there's a gap.
        // If the heap is empty, all workers must have finished so return.
        loop {
            let min_frame = match min_heap.pop() { Some(r) => r, _ => return };

            if min_frame.0.frame_number == expected_frame {
                process_function(min_frame.0.0);
                expected_frame += 1;
                found_a_frame = true;
            } else {
                min_heap.push(min_frame); // Put the frame back.
                break;
            }
        }

        if found_a_frame { continue; }

        // If we didn't find a frame then some compressed data is missing.
        // This isn't the same as frames being dropped from the capture.
        process_function(crate::StreamFrame {
            status: crate::FrameStatus::Missing,
            image_data: None,
            frame_number: expected_frame,
            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            ..Default::default()
        });

        expected_frame += 1;
    }
}

struct OrderableFrame(crate::StreamFrame);

impl ops::Deref for OrderableFrame {
    type Target = crate::StreamFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl cmp::Ord for OrderableFrame {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.frame_number.cmp(&other.frame_number)
    }
}

impl cmp::PartialOrd for OrderableFrame {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.frame_number.partial_cmp(&other.frame_number)
    }
}

impl cmp::PartialEq for OrderableFrame {
    fn eq(&self, other: &Self) -> bool {
        self.frame_number.eq(&other.frame_number)
    }
}

impl cmp::Eq for OrderableFrame {}
