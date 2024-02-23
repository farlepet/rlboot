pub mod fat;
use core::slice;
use std::fmt::Error;

use std::fs::File;
use std::io::Write;
use std::{mem, path::Path, env};
use std::mem::MaybeUninit;

use fat::{FAT, FatDataBootsector};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        usage();
        return Err(Error);
    }
    let stage: u8 = args[1].parse().unwrap();
    if (stage < 1) || (stage > 2) {
        usage();
        return Err(Error);
    }

    let fatfs = FAT::open(Path::new(&args[2]), stage == 2).unwrap();
    match stage {
        1 => stage1(&fatfs, &args),
        2 => stage2(&fatfs, &args),
        _ => Err(Error)
    }
}

fn usage() {
    eprintln!("USAGE: ");
    eprintln!("  sector_mapper 1 <floppy image> <stage 2 filename> <map file>");
    eprintln!("  sector_mapper 2 <floppy image> <map file>");
}

fn stage1(fs: &FAT, args: &Vec<String>) -> Result<(), Error> {
    if args.len() != 5 {
        usage();
        return Err(Error);
    }

    let mut map_file = File::create(Path::new(&args[4])).unwrap();
    let s2file = fs.find_file(None, &args[3]).unwrap();

    let clusters = fs.get_file_clusters(&s2file).unwrap();

    let mut chunks: Vec<SectorMapChunk> = vec!();
    let mut chunk = SectorMapChunk::default();
    chunk.entries[0] = SectorMapEntry {
        sector: ((clusters[0] as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16,
        count:  1
    };
    let mut cmap = 0;

    for i in 1..clusters.len() {
        if clusters[i] != (clusters[i-1] + fs.get_cluster_size() as u32) {
            cmap += 1;
            if cmap >= SECTOR_MAP_ENTRIES {
                cmap = 0;
                chunks.push(chunk);
            }

            chunk.entries[cmap] = SectorMapEntry {
                sector: ((clusters[i] as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16,
                count:  1
            };
        } else {
            chunk.entries[cmap].count += 1;
        }
    }
    chunks.push(chunk);

    let ptr = chunks.as_ptr().cast();
    let len = chunks.len() * mem::size_of::<SectorMapChunk>();

    let data = unsafe { slice::from_raw_parts(ptr, len) };

    if map_file.write(data).is_err() {
        Err(Error)
    } else {
        Ok(())
    }
}

fn stage2(fs: &FAT, args: &Vec<String>) -> Result<(), Error> {
    if args.len() != 4 {
        usage();
        return Err(Error);
    }

    let mapfile = fs.find_file(None, &args[3]).unwrap();

    let clusters = fs.get_file_clusters(&mapfile).unwrap();

    let mut off = mem::offset_of!(FatDataBootsector, stage2_map_sector);
    for clust in clusters {
        let real_clust = ((clust as usize / fs.get_cluster_size()) * fs.get_sectors_per_cluster()) as u16;
        if fs.write_u16(off, real_clust).is_err() { return Err(Error); }
        off = clust as usize + mem::offset_of!(SectorMapChunk, next_chunk);
    }

    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SectorMapEntry {
    sector: u16,
    count:  u16,
}

const SECTOR_MAP_ENTRIES: usize = (512 - 4) / mem::size_of::<SectorMapEntry>();

#[repr(C)]
#[derive(Clone, Copy)]
struct SectorMapChunk {
    entries: [SectorMapEntry; SECTOR_MAP_ENTRIES],
    next_chunk: u16,
    _reserved: u16,
}

impl Default for SectorMapChunk {
    fn default() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}
